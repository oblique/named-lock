use libc::{LOCK_EX, LOCK_NB, LOCK_UN};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

use crate::error::*;

#[derive(Debug)]
pub(crate) struct RawNamedLock {
    lock_file: File,
}

impl RawNamedLock {
    pub(crate) fn create(lock_path: &Path) -> Result<RawNamedLock> {
        let lock_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .or_else(|_| OpenOptions::new().write(true).open(&lock_path))
            .map_err(Error::CreateFailed)?;

        Ok(RawNamedLock {
            lock_file,
        })
    }

    pub(crate) fn try_lock(&self) -> Result<()> {
        unsafe { flock(self.lock_file.as_raw_fd(), LOCK_EX | LOCK_NB) }
    }

    pub(crate) fn lock(&self) -> Result<()> {
        unsafe { flock(self.lock_file.as_raw_fd(), LOCK_EX) }
    }

    pub(crate) fn unlock(&self) -> Result<()> {
        unsafe { flock(self.lock_file.as_raw_fd(), LOCK_UN) }
    }
}

unsafe fn flock(fd: RawFd, operation: i32) -> Result<()> {
    loop {
        let rc = libc::flock(fd, operation);

        if rc < 0 {
            let err = io::Error::last_os_error();

            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            } else if err.kind() == io::ErrorKind::WouldBlock {
                return Err(Error::WouldBlock);
            } else if (operation & LOCK_EX) == LOCK_EX {
                return Err(Error::LockFailed);
            } else if (operation & LOCK_UN) == LOCK_UN {
                return Err(Error::UnlockFailed);
            }
        }

        break;
    }

    Ok(())
}
