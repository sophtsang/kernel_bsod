#![cfg_attr(not(test), no_std)]
use core::{alloc::Layout, cmp::max, ptr::NonNull};

pub struct FreeHole {
    // defines a free space in memory, end of the hole is allocated space, points to the address of the next contiguous hold
    // metadata occupies the beginning of the free region
    size: usize, // size of this free hole, adjacent is allocated
    next_free: Option<NonNull<FreeHole>>, // pointer to next hole
    // whatever is not taken up by size(size + next_free) is available bytes for allocations
}

impl FreeHole {
    const fn new(size: usize) -> Self {
        Self {
            size: size,
            next_free: None
        }
    }

    // frees the block_size bytes at ptr, walking from self: any free hole (or the dummy head)
    // that lies before ptr. Err if the block overlaps a free hole (double free or wrong layout)
    pub fn deallocate(&mut self, ptr: NonNull<u8>, block_size: usize) -> Result<(), ()> {
        let mut prev = self as *mut FreeHole;
        let mut curr_opt = self.next_free;
        let addr = ptr.as_ptr() as usize;
        while let Some(mut curr_ptr) = curr_opt && (curr_ptr.as_ptr() as usize) < addr {
            // traversing until find curr_ptr > ptr
            let curr = unsafe { curr_ptr.as_mut() }; // &mut FreeHole
            prev = curr;
            curr_opt = curr.next_free;
        }

        // the cases
        let curr = curr_opt.map(|mut p| unsafe { p.as_mut() });
        let curr_ptr = curr_opt.map_or(0, |p| p.as_ptr() as usize);
        let prev_ptr = prev as usize;
        // real holes are always >= size_of::<FreeHole>(), so size 0 means prev is the dummy head,
        // which is never merged into
        let prev_size = unsafe { (*prev).size };

        let left_free = prev_size > 0 && prev_ptr + prev_size == addr;
        let right_free = curr.is_some() && curr_ptr == addr + block_size;

        if (prev_size > 0 && prev_ptr + prev_size > addr)
            || (curr.is_some() && addr + block_size > curr_ptr) {
            // thewe are pointing to FreeHole
            return Err(());
        }

        // we know [prev][...][Alloc][...][curr] now, s.t. we can do linked
        // list shenanigans
        // case 1: both neighbors are free
        match (left_free, right_free) {
            (true, true) => {
                // we have [prev][Alloc][curr] => [     free     ]
                unsafe { (*prev).size += block_size + curr.as_ref().unwrap().size;
                (*prev).next_free = curr.as_ref().unwrap().next_free; }
            }
            (true, false) => {
                // we have [prev][Alloc][Alloc][curr] => [  free  ][Alloc][curr]
                // literally prev just swallows alloc
                unsafe { (*prev).size += block_size; }
            }
            (false, true) => {
                // we have [prev][Alloc][Alloc][curr] => [prev][Alloc][  free  ]
                // literally curr swallows alloc
                let mut new_free = FreeHole::new(block_size + curr.as_ref().unwrap().size);
                unsafe { new_free.next_free = curr.as_ref().unwrap().next_free;
                        (ptr.as_ptr() as *mut FreeHole).write(new_free);
                        (*prev).next_free = NonNull::new(ptr.as_ptr() as *mut FreeHole) };
            }
            (false, false) => {
                // we have [prev][Alloc][Alloc][Alloc][curr] => [prev][Alloc][free][Alloc][curr]
                let mut new_free = FreeHole::new(block_size);
                unsafe { new_free.next_free = curr_opt;
                        (ptr.as_ptr() as *mut FreeHole).write(new_free);
                        (*prev).next_free = NonNull::new(ptr.as_ptr() as *mut FreeHole) };
            }
        }
        Ok(())
    }

    pub fn allocate(&mut self, mut prev: *mut FreeHole, size: usize, align: usize) -> Option<NonNull<u8>> {
        let align = max(align, align_of::<FreeHole>());
        let mut curr_opt = Some(NonNull::from(self));
        while let Some(mut curr_ptr) = curr_opt { // iterating through the FreeHoles
            let curr = unsafe { curr_ptr.as_mut() }; // &mut FreeHole
            let curr_ptr = curr as *mut FreeHole as usize; // mem addr of this FreeHole
            // ALIGNMENT BS
            let mut aligned_ptr = Memory::get_aligned(curr_ptr, align);
            if curr_ptr != aligned_ptr && aligned_ptr - curr_ptr < size_of::<FreeHole>() { 
                // check whether has space for FreeHole metadata w/ size size_of::<FreeHole>()
                aligned_ptr = Memory::get_aligned(curr_ptr + size_of::<FreeHole>(), align);
            }
            // a possible allocated addr and available size
            let alloc_front = aligned_ptr - curr_ptr;
            let free_size = curr.size.saturating_sub(alloc_front); // available size to be allocated

            if free_size == size || (free_size > size && free_size - size >= size_of::<FreeHole>()) {
                if alloc_front > 0 {
                    curr.size = alloc_front;
                    prev = curr;
                }
                if free_size == size {
                    // no free block only [Alloc]
                    unsafe { (*prev).next_free = curr.next_free }
                } else {
                    // split into [Alloc][FreeHole  ]
                    let mut split_free = FreeHole::new(free_size - size);
                    split_free.next_free = curr.next_free;

                    let ptr = (aligned_ptr + size) as *mut FreeHole; // new free
                    unsafe { ptr.write(split_free) };
                    unsafe { (*prev).next_free = NonNull::new(ptr) };
                }
                return NonNull::new(aligned_ptr as *mut u8);
            }
            prev = curr;
            curr_opt = curr.next_free;
        }
        None
    }  
}

pub struct Memory {
    free_head: FreeHole,
    alloc_size: usize, // total size of all allocated portions of memory
    // structured somewhat like a linked list
    mem_start: usize,
    mem_end: usize,
}

impl Memory {
    // ig, should this check whether there are any free holes in memory first -> then first-fit algo to find the first free block that has size >= process size
    // Memory will always include a dummy FreeHole node at the beginning with size 0
    pub const fn new() -> Self {
        Self {
            free_head: FreeHole::new(0),
            alloc_size: 0,
            mem_start: 0,
            mem_end: 0
        }
    }

    // get val greater than or equal to val that aligns with FreeHole's alignment
    fn get_aligned(val: usize, align: usize) -> usize {
        (val + align - 1) & !(align - 1)
    }

    // allocating finds a FreeHole, possibly splits it into [Alloc][Subset of FreeHole] 
    // memory starts as:
    //         [FreeHole                            ]
    // unsafe cuz deals with raw pointers
    // caller must guaratee that new() has already been called.
    pub unsafe fn init(&mut self, start_addr: usize, size: usize) {
        // start_addr must actually start at some addr that is a multiple of FreeHole's alignment
        debug_assert!(self.mem_end == 0);
        let align = align_of::<FreeHole>();
        let aligned_addr = Self::get_aligned(start_addr, align);

        let mut aligned_size = size - (aligned_addr - start_addr); //could have skipped bytes that would not count to available size
        aligned_size -= aligned_size % align; // to make sure both start and end addr are in alignment
        assert!(aligned_size >= size_of::<FreeHole>());

        self.mem_start = aligned_addr; // the start addr after alignment
        self.mem_end = aligned_addr + aligned_size;

        let ptr = aligned_addr as *mut FreeHole; // this pointer is pointing to the address aligned_addr
        unsafe { ptr.write(FreeHole::new(aligned_size)) };
        // the dummy head is now contain the addr of the actual first free head
        self.free_head.next_free = NonNull::new(ptr);
    }

    // an example of how allocation modifies memory distribution:
    // allocating finds a FreeHole, possibly splits it into [Alloc][Subset of FreeHole] 
    // memory starts as:
    //         [FreeHole                            ], after some allocs, we get:
    // before: [FreeHole][Alloc][FreeHole           ], alloc [Alloc    ]
    // after:  [FreeHole][Alloc][Alloc    ][FreeHole]
    // directly modifies Memory
    // returns addr where we allocated layout, if possible
    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        // first fit allocation: 
        let block_size = Self::get_aligned(max(layout.size(), size_of::<FreeHole>()), align_of::<FreeHole>());
        // all blocks in Memory are guaranteed to be aligned
        if let Some(mut first_free) = self.free_head.next_free {
            // deference the pointer
            if let Some(addr) = unsafe { first_free.as_mut().allocate(&mut self.free_head as *mut FreeHole, block_size, layout.align()) } {
                self.alloc_size += block_size;
                return Ok(addr);
            }
        }
        Err(())
    }

    // deallocating an allocated block: [Alloc], given its addr
    // before: [FreeHole][Alloc][Alloc    ][FreeHole]
    // after:  [FreeHole       ][Alloc    ][FreeHole]
    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // no need to align, all alloc addrs should be returned via some
        // prior allocate call, meaning that the addr is aligned
        let block_size = Self::get_aligned(max(layout.size(), size_of::<FreeHole>()), align_of::<FreeHole>());
        // start the walk at the dummy head: covers every hole, and works when the list is empty
        match self.free_head.deallocate(ptr, block_size) {
            Ok(_) => { self.alloc_size -= block_size; },
            Err(_) => {}
        }
    }
}

// SpinLock allows T: Send only. sound: the raw pointers only point into the heap region this
// Memory owns, and the lock serializes every access to them
unsafe impl Send for Memory {}