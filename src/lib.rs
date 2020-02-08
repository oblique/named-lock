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
    use std::env;
    use std::process::{Child, Command};
    use std::thread::sleep;
    use std::time::Duration;
    use uuid::Uuid;

    fn call_proc_num(num: u32, uuid: &str) -> Child {
        let exe = env::current_exe().expect("no exe");
        let mut cmd = Command::new(exe);

        cmd.env("TEST_CROSS_PROCESS_LOCK_PROC_NUM", num.to_string())
            .env("TEST_CROSS_PROCESS_LOCK_UUID", uuid)
            .arg("tests::cross_process_lock")
            .spawn()
            .unwrap()
    }

    #[test]
    fn cross_process_lock() -> Result<()> {
        let proc_num = env::var("TEST_CROSS_PROCESS_LOCK_PROC_NUM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let uuid = env::var("TEST_CROSS_PROCESS_LOCK_UUID")
            .unwrap_or_else(|_| Uuid::new_v4().to_hyphenated().to_string());

        match proc_num {
            0 => {
                let mut handle1 = call_proc_num(1, &uuid);
                sleep(Duration::from_millis(100));

                let mut handle2 = call_proc_num(2, &uuid);
                sleep(Duration::from_millis(200));

                let lock = NamedLock::create(&uuid)?;
                assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
                lock.lock().expect("failed to lock");

                assert!(handle2.wait().unwrap().success());
                assert!(handle1.wait().unwrap().success());
            }
            1 => {
                let lock =
                    NamedLock::create(&uuid).expect("failed to create lock");

                let _guard = lock.lock().expect("failed to lock");
                assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
                sleep(Duration::from_millis(200));
            }
            2 => {
                let lock =
                    NamedLock::create(&uuid).expect("failed to create lock");

                assert_matches!(lock.try_lock(), Err(Error::WouldBlock));
                let _guard = lock.lock().expect("failed to lock");
                sleep(Duration::from_millis(300));
            }
            _ => unreachable!(),
        }

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
