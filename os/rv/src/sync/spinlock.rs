use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T> {
    locked: AtomicBool, // cs4414 implements this atomic lock as AtomicUsize::new(1)
    data: UnsafeCell<T>,
}

// the lock is what makes sharing `data` across harts/interrupts sound
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        loop {
            // so like keep compare_exchanging until acquired lock
            match self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return SpinLockGuard { lock: self }, // return guard
                Err(_) => {
                    while self.locked.load(Ordering::Relaxed) {
                        core::hint::spin_loop();
                    }
                }
            }
        }
    }
}

// guard stuff, needs &, *, &mut, and drop
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}
