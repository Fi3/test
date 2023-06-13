use std::sync::{Arc, Mutex as Mutex_, MutexGuard};

pub enum Inner<'a> {
    Ref(&'a u8),
    Owned(Vec<u8>),
}

struct PanicableStruct<'a> {
    _a: Vec<Inner<'a>>, // Comment this line in order to be able to build the crate
    _b: Vec<Inner<'a>>,
}

#[inline(never)]
fn new_fs(_uno: Vec<PanicableStruct<'static>>) {}

#[inline(never)]
fn apply(x: Arc<Mutex<()>>) {
    x.safe_lock(|_s| {
        let uno: Vec<PanicableStruct<'static>> = vec_new();
        new_fs(uno);
    });
}

fn main() {
    let x = Arc::new(Mutex::new(()));
    apply(x);
}

/// This allow to initialize a vector inside a safe_lock. It just abort the process if the vector
/// initialization fail, for that can no panic.
pub fn vec_new<T>() -> Vec<T> {
    match std::panic::catch_unwind(|| Vec::new()).map_err(|_| std::process::abort()) {
        Ok(r) => r,
        Err(_) => std::process::abort(),
    }
}

/// Safer Mutex wrapper
pub struct Mutex<T: ?Sized>(Mutex_<T>);

impl<T> Mutex<T> {
    /// `safe_lock` takes a closure that takes a mutable reference to the inner value, and returns the
    ///  value of the closure.
    /// This is used to:
    ///     * ensure no async executions while locked.
    ///     * make dead lock less likley
    ///     * prevent `PoisonLock`
    ///
    /// To prevent `PoisonLock` errors, the closure can not panic (statically checked).
    ///
    /// Arguments:
    ///
    /// * `thunk`: A closure that takes a mutable reference to the value inside the Mutex and returns a
    /// value of type Ret.
    ///
    /// This function in splitted in 3 part safe_lock lock and execute just to better analyze the
    /// builded assembly code
    #[inline(never)]
    pub fn safe_lock<F, Ret>(&self, thunk: F) -> Ret
    where
        F: FnOnce(&mut T) -> Ret,
    {
        {
            let mut lock = self.lock();
            let mut __guard = __NoPanic;
            let return_value = self.execute(thunk, &mut lock);
            drop(lock);
            core::mem::forget(__guard);
            return_value
        }
    }

    #[inline(never)]
    pub fn lock(&self) -> MutexGuard<T> {
        match self.0.lock() {
            Ok(r) => r,
            Err(_) => std::process::abort(),
        }
    }

    #[inline(never)]
    pub fn execute<F, Ret>(&self, thunk: F, lock: &mut MutexGuard<T>) -> Ret
    where
        F: FnOnce(&mut T) -> Ret,
    {
        thunk(&mut *lock)
    }

    pub fn new(v: T) -> Self {
        Mutex(Mutex_::new(v))
    }
}

// based on https://github.com/dtolnay/no-panic
pub struct __NoPanic;
extern "C" {
    #[link_name = "safe_lock called on a function that may panic"]
    fn trigger() -> !;
}

impl core::ops::Drop for __NoPanic {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            trigger();
        }
    }
}
