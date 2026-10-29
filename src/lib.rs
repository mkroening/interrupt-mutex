//! A mutex for sharing data with interrupt handlers.
//!
//! Using normal mutexes to share data with interrupt handlers may result in deadlocks.
//! This is because interrupts may be raised while the mutex is being held on the same thread.
//!
//! [`InterruptMutex`] wraps another mutex and disables interrupts while the inner mutex is locked.
//! When the mutex is unlocked, the previous interrupt state is restored.
//! This makes [`InterruptMutex`] suitable for sharing data with interrupts.
//!
//! When used in bare-metal environments with spinlocks, locking the mutex corresponds to Linux's `spin_lock_irqsave` and unlocking corresponds to `spin_unlock_irqrestore`.
//! See the [Unreliable Guide To Locking — The Linux Kernel documentation].
//! While `spin_lock_irqsave(lock, flags)` saves the interrupt flags in the explicit `flags` argument, [`InterruptMutex`] saves the interrupt flags internally.
//!
//! [Unreliable Guide To Locking — The Linux Kernel documentation]: https://www.kernel.org/doc/html/latest/kernel-hacking/locking.html#locking-between-hard-irq-and-softirqs-tasklets
//!
//! [Drop Order]: #caveats
//!
//! # Caveats
//!
//! <div class="warning">Interrupts are disabled on a best-effort basis.</div>
//!
//! Holding an [`InterruptMutexGuard`] does not guarantee that interrupts are disabled.
//! Dropping guards from different [`InterruptMutex`]es in the wrong order might enable interrupts prematurely.
//! Similarly, you can just enable interrupts manually while holding a guard.
//!
//! # Examples
//!
//! ```no_run
//! // Make a mutex of your choice into an `InterruptMutex`.
//! type InterruptMutex<T> = interrupt_mutex::InterruptMutex<parking_lot::RawMutex, T>;
//!
//! static X: InterruptMutex<Vec<i32>> = InterruptMutex::new(Vec::new());
//!
//! fn interrupt_handler() {
//!     X.lock().push(1);
//! }
//! #
//! # fn raise_interrupt() {}
//!
//! let v = X.lock();
//! // Raise an interrupt
//! raise_interrupt();
//! assert_eq!(*v, vec![]);
//! drop(v);
//!
//! // The interrupt handler runs
//!
//! let v = X.lock();
//! assert_eq!(*v, vec![1]);
//! drop(v);
//! ```

#![no_std]

use lock_api::RawMutex;

/// A mutex for sharing data with interrupt handlers.
///
/// This mutex wraps another [`RawMutex`] and disables interrupts while locked.
pub struct RawInterruptMutex<I> {
    inner: I,
}

// SAFETY: The `UnsafeCell` is locked by `inner`, initialized on `lock` and uninitialized on `unlock`.
unsafe impl<I: Sync> Sync for RawInterruptMutex<I> {}
// SAFETY: Mutexes cannot be send to other threads while locked.
// Sending them while unlocked is fine.
unsafe impl<I: Send> Send for RawInterruptMutex<I> {}

pub struct InterruptGuard<I> {
    interrupt_guard: interrupts::Guard,
    inner: I,
}

unsafe impl<I: RawMutex> RawMutex for RawInterruptMutex<I> {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self { inner: I::INIT };

    type Guard = InterruptGuard<I::Guard>;

    #[inline]
    fn lock(&self) -> Self::Guard {
        let interrupt_guard = interrupts::disable();
        let inner = self.inner.lock();
        Self::Guard {
            interrupt_guard,
            inner,
        }
    }

    #[inline]
    fn try_lock(&self) -> Option<Self::Guard> {
        let interrupt_guard = interrupts::disable();
        self.inner.try_lock().map(|inner| Self::Guard {
            interrupt_guard,
            inner,
        })
    }

    #[inline]
    unsafe fn unlock(&self, guard: Self::Guard) {
        unsafe {
            self.inner.unlock(guard.inner);
        }
        drop(guard.interrupt_guard)
    }

    #[inline]
    fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }
}

/// A [`lock_api::Mutex`] based on [`RawInterruptMutex`].
pub type InterruptMutex<I, T> = lock_api::Mutex<RawInterruptMutex<I>, T>;

/// A [`lock_api::MutexGuard`] based on [`RawInterruptMutex`].
pub type InterruptMutexGuard<'a, I, T> = lock_api::MutexGuard<'a, RawInterruptMutex<I>, T>;
