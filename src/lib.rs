//! This crate provides a simple and cross-platform implementation of named locks.
//! You can use this to lock critical sections between processes.
//!
//! ## Example
//!
//! ```rust
//! use named_lock::NamedLock;
//! use named_lock::Result;
//!
//! fn main() -> Result<()> {
//!     let lock = NamedLock::create("foobar")?;
//!     let _guard = lock.lock()?;
//!
//!     // Do something...
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Implementation
//!
//! On UNIX systems this is implemented by using files and [`flock`]. The path of
//! the created lock file will be `/tmp/<name>.lock`.
//!
//! On Windows this is implemented by creating named mutex ([`CreateMutexW`]).
//!
//!
//! [`flock`]: https://linux.die.net/man/2/flock
//! [`CreateMutexW`]: https://docs.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexw

use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

mod error;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use crate::error::*;
#[cfg(unix)]
use crate::unix::RawNamedLock;
#[cfg(windows)]
use crate::windows::RawNamedLock;

// We handle two edge cases:
//
// On UNIX systems, after locking a file descriptor you can lock it again
// as many times you want. However OS does not keep a counter, so only one
// unlock must be performed. To avoid re-locking, we guard it with real mutex.
//
// On Windows systems, after locking a `HANDLE` you can create another
// `HANDLE` for the same named lock and the same process and Windows
// will allow you to re-lock it. To avoid this, we ensure that one `HANDLE`
// exists in each process for each name.
static OPENED_RAW_LOCKS: Lazy<
    Mutex<HashMap<String, Weak<Mutex<RawNamedLock>>>>,
> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct NamedLock {
    raw: Arc<Mutex<RawNamedLock>>,
}

impl NamedLock {
    pub fn create(name: &str) -> Result<NamedLock> {
        let mut opened_locks = OPENED_RAW_LOCKS.lock();

        let lock = match opened_locks.get(name).and_then(|x| x.upgrade()) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(Mutex::new(RawNamedLock::create(name)?));
                opened_locks.insert(name.to_owned(), Arc::downgrade(&lock));
                lock
            }
        };

        Ok(NamedLock {
            raw: lock,
        })
    }

    pub fn try_lock<'r>(&'r self) -> Result<NamedLockGuard<'r>> {
        let guard = self.raw.try_lock().ok_or(Error::WouldBlock)?;

        guard.try_lock()?;

        Ok(NamedLockGuard {
            raw: guard,
        })
    }

    pub fn lock<'r>(&'r self) -> Result<NamedLockGuard<'r>> {
        let guard = self.raw.lock();

        guard.lock()?;

        Ok(NamedLockGuard {
            raw: guard,
        })
    }
}

#[derive(Debug)]
pub struct NamedLockGuard<'r> {
    raw: MutexGuard<'r, RawNamedLock>,
}

impl<'r> Drop for NamedLockGuard<'r> {
    fn drop(&mut self) {
        let _ = self.raw.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::assert_matches;
    use uuid::Uuid;

    #[test]
    fn check_lock() -> Result<()> {
        let uuid = Uuid::new_v4().to_hyphenated().to_string();
        let lock1 = NamedLock::create(&uuid)?;
        let lock2 = NamedLock::create(&uuid)?;

        {
            let _guard1 = lock1.lock()?;
            assert_matches!(lock1.try_lock(), Err(Error::WouldBlock));
            assert_matches!(lock2.try_lock(), Err(Error::WouldBlock));
        }

        {
            let _guard2 = lock2.lock()?;
            assert_matches!(lock1.try_lock(), Err(Error::WouldBlock));
            assert_matches!(lock2.try_lock(), Err(Error::WouldBlock));
        }

        Ok(())
    }

    #[test]
    fn check_try_lock() -> Result<()> {
        let uuid = Uuid::new_v4().to_hyphenated().to_string();
        let lock1 = NamedLock::create(&uuid)?;
        let lock2 = NamedLock::create(&uuid)?;

        {
            let _guard1 = lock1.try_lock()?;
            assert_matches!(lock1.try_lock(), Err(Error::WouldBlock));
            assert_matches!(lock2.try_lock(), Err(Error::WouldBlock));
        }

        {
            let _guard2 = lock2.try_lock()?;
            assert_matches!(lock1.try_lock(), Err(Error::WouldBlock));
            assert_matches!(lock2.try_lock(), Err(Error::WouldBlock));
        }

        Ok(())
    }
}
