//! This crate provides a simple and cross-platform implementation of named locks.
//! You can use this to lock sections between processes.
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
//! On Windows this is implemented by creating named mutex with [`CreateMutexW`].
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
// On Windows, after locking a `HANDLE` you can create another `HANDLE` for
// the same named lock and the same process and Windows will allow you to
// re-lock it. To avoid this, we ensure that one `HANDLE` exists in each
// process for each name.
static OPENED_RAW_LOCKS: Lazy<
    Mutex<HashMap<String, Weak<Mutex<RawNamedLock>>>>,
> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Cross-process lock that is identified by name.
#[derive(Debug)]
pub struct NamedLock {
    raw: Arc<Mutex<RawNamedLock>>,
}

impl NamedLock {
    /// Create/open a named lock.
    ///
    /// On UNIX systems this will create/open a file at `/tmp/<name>.lock`.
    ///
    /// On Windows this will create/open a named mutex.
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

    /// Try to lock named lock.
    ///
    /// If it is already locked, `Error::WouldBlock` will be returned.
    pub fn try_lock<'r>(&'r self) -> Result<NamedLockGuard<'r>> {
        let guard = self.raw.try_lock().ok_or(Error::WouldBlock)?;

        guard.try_lock()?;

        Ok(NamedLockGuard {
            raw: guard,
        })
    }

    /// Lock named lock.
    pub fn lock<'r>(&'r self) -> Result<NamedLockGuard<'r>> {
        let guard = self.raw.lock();

        guard.lock()?;

        Ok(NamedLockGuard {
            raw: guard,
        })
    }
}

/// Scoped guard that unlocks NamedLock.
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
    use std::thread::sleep;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn cross_process_lock() -> Result<()> {
        mitosis::init();
        let uuid = Uuid::new_v4().to_hyphenated().to_string();

        let handle1 = mitosis::spawn(uuid.clone(), |uuid| {
            let lock = NamedLock::create(&uuid).expect("failed to create lock");
            let _guard = lock.lock().expect("failed to lock");

            assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
            sleep(Duration::from_millis(300));
        });
        sleep(Duration::from_millis(100));

        let handle2 = mitosis::spawn(uuid.clone(), |uuid| {
            let lock = NamedLock::create(&uuid).expect("failed to create lock");
            assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
            lock.lock().unwrap();
        });
        sleep(Duration::from_millis(100));

        let lock = NamedLock::create(&uuid)?;
        assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
        lock.lock().unwrap();

        handle1.join().unwrap();
        handle2.join().unwrap();
        lock.try_lock().unwrap();

        Ok(())
    }

    #[test]
    fn edge_cases() -> Result<()> {
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
