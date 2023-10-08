//! A mutex for sharing data with interrupt handlers or signal handlers.
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
//! ```
//! // Make a mutex of your choice into an `InterruptMutex`.
//! type InterruptMutex<T> = interrupt_mutex::InterruptMutex<parking_lot::RawMutex, T>;
//!
//! static X: InterruptMutex<Vec<i32>> = InterruptMutex::new(Vec::new());
//!
//! fn interrupt_handler() {
//!     X.lock().push(1);
//! }
//! #
//! # // Setup signal handling for demo
//! #
//! # use nix::libc;
//! # use nix::sys::signal::{self, SigHandler, Signal};
//! #
//! # extern "C" fn handle_sigint(_signal: libc::c_int) {
//! #     interrupt_handler();
//! # }
//! #
//! # let handler = SigHandler::Handler(handle_sigint);
//! # unsafe { signal::signal(Signal::SIGINT, handler) }.unwrap();
//! #
//! # fn raise_interrupt() {
//! #     signal::raise(Signal::SIGINT);
//! # }
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

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use lock_api::{GuardNoSend, RawMutex};

/// A mutex for sharing data with interrupt handlers or signal handlers.
///
/// This mutex wraps another [`RawMutex`] and disables interrupts while locked.
pub struct RawInterruptMutex<I> {
    inner: I,
    interrupt_guard: UnsafeCell<MaybeUninit<interrupts::Guard>>,
}

// SAFETY: The `UnsafeCell` is locked by `inner`, initialized on `lock` and uninitialized on `unlock`.
unsafe impl<I: Sync> Sync for RawInterruptMutex<I> {}
// SAFETY: Mutexes cannot be send to other threads while locked.
// Sending them while unlocked is fine.
unsafe impl<I: Send> Send for RawInterruptMutex<I> {}

unsafe impl<I: RawMutex> RawMutex for RawInterruptMutex<I> {
    const INIT: Self = Self {
        inner: I::INIT,
        interrupt_guard: UnsafeCell::new(MaybeUninit::uninit()),
    };

    type GuardMarker = GuardNoSend;

    #[inline]
    fn lock(&self) {
        let guard = interrupts::disable();
        self.inner.lock();
        // SAFETY: We have exclusive access through locking `inner`.
        unsafe {
            self.interrupt_guard.get().write(MaybeUninit::new(guard));
        }
    }

    #[inline]
    fn try_lock(&self) -> bool {
        let guard = interrupts::disable();
        let ok = self.inner.try_lock();
        if ok {
            // SAFETY: We have exclusive access through locking `inner`.
            unsafe {
                self.interrupt_guard.get().write(MaybeUninit::new(guard));
            }
        }
        ok
    }

    #[inline]
    unsafe fn unlock(&self) {
        // SAFETY: We have exclusive access through locking `inner`.
        let guard = unsafe { self.interrupt_guard.get().replace(MaybeUninit::uninit()) };
        // SAFETY: `guard` was initialized when locking.
        let guard = unsafe { guard.assume_init() };
        unsafe {
            self.inner.unlock();
        }
        drop(guard);
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
