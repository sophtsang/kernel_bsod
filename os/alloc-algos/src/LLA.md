Rust's ```linked_list_allocator``` provides a heap allocator without depending on the Rust standard library, operating system, or dynamic memory allocation.
Maintains a contigous region of memory provided to it by the kernel, using an intrusive linked list of free memory regions. 

Need to implement a first-fit linked-list heap allocator.

*First Fit algorithm*: allocates the first available memory block that is large enough to accomodate a process.
When implemented as a linked list, maintains two lists:
1) *Free list*: tracks available memory blocks
    - Keeps track of which portions of the heap are currently available
2) *Allocated list*: tracks processes and their assigned blocks
    - Exposes allocation and deallocation to Rust
    - ```free_list``` and ```alloc_list``` have no intersecting elements

Dynamic allocation and deallocation occurs by updating linked list nodes.

Parts of the ```linked_list_allocator``` that will be implemented here:
1) *Initialization*: create a fully free ```pub struct Memory```, in which memory starts out completely free, just one large ```FreeHole``` block, and without any allocations.
    - Let's have ```new(size: usize)``` do this.
2) *Allocation*: traverse ```Memory```, allocate the first free block ```FreeHole``` large enough for the process -> split the free block into ```alloc``` with its process or tag id, and a reduced size ```FreeHole```.
3) *Deallocation*: locate the process in the allocated list by tag id, turn the ```alloc``` block into ```FreeHole```, also ig check whether neighbors on both sides are ```FreeHole```s or not to determine whether to merge into a larger ```FreeHole```. 
    - In this case, ig we have to keep track of previous ```FreeHole```, and also check next adjacent block when we find the corresponding ```alloc``` to deallocate. 

```rust
use core::alloc::Layout;

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

    pub fn traverse_on_id(&self, id) -> Option<NonNull<usize>> {
        // if self.id == id {
        //     return Some(self)
        // }
        // if let Some(next) = self.next {
        //     next.traverse_on_id(id);
        // }
        // None
    }

    pub fn traverse_on_size(&mut self, prev: *mut FreeHole, size: usize, align: usize) -> Option<NonNull<u8>> {
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
        aligned_size -= (aligned_size % align); // to make sure both start and end addr are in alignment
        assert!(aligned_size >= size_of::<FreeHole>());

        self.mem_start = aligned_addr; // the start addr after alignment
        self.mem_end = aligned_addr + aligned_size;

        let ptr = aligned_addr as *const mut FreeHole; // this pointer is pointing to the address aligned_addr
        ptr.write(FreeHole::new(aligned_size));
        // the dummy head is now contain the addr of the actual first free head
        self.free_head.next_free = Some(NonNull::new(unsafe { ptr }));
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
            if let Some(addr) = unsafe { first_free.as_mut().traverse_on_size(&mut self.free_head as *mut FreeHole, block_size, layout.align()) } {
                self.alloc_size += block_size;
                return Ok(addr);
            }
        }
        Err(())
    }

    // deallocating an allocated block: [Alloc], each alloc has an id
    // before: [FreeHole][Alloc][Alloc    ][FreeHole]
    // after:  [FreeHole       ][Alloc    ][FreeHole]
    pub unsafe fn deallocate(&mut self, layout: Layout) -> bool {
        
    }
}
```