use std::io;
use std::ptr;
use widestring::WideCString;
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::handleapi::CloseHandle;
use winapi::um::synchapi::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use winapi::um::winbase::{INFINITE, WAIT_ABANDONED, WAIT_OBJECT_0};
use winapi::um::winnt::HANDLE;

use crate::error::*;

#[derive(Debug)]
pub(crate) struct RawNamedLock {
    handle: HANDLE,
}

unsafe impl Sync for RawNamedLock {}
unsafe impl Send for RawNamedLock {}

impl RawNamedLock {
    pub(crate) fn create(name: &str) -> Result<RawNamedLock> {
        let name = WideCString::from_str(name).unwrap();
        let handle = unsafe { CreateMutexW(ptr::null_mut(), 0, name.as_ptr()) };

        if handle.is_null() {
            Err(Error::CreateFailed(io::Error::last_os_error()))
        } else {
            Ok(RawNamedLock {
                handle,
            })
        }
    }

    pub(crate) fn try_lock(&self) -> Result<()> {
        let rc = unsafe { WaitForSingleObject(self.handle, 0) };

        if rc == WAIT_OBJECT_0 || rc == WAIT_ABANDONED {
            Ok(())
        } else if rc == WAIT_TIMEOUT {
            Err(Error::WouldBlock)
        } else {
            Err(Error::LockFailed)
        }
    }

    pub(crate) fn lock(&self) -> Result<()> {
        let rc = unsafe { WaitForSingleObject(self.handle, INFINITE) };

        if rc == WAIT_OBJECT_0 || rc == WAIT_ABANDONED {
            Ok(())
        } else {
            Err(Error::LockFailed)
        }
    }

    pub(crate) fn unlock(&self) -> Result<()> {
        let rc = unsafe { ReleaseMutex(self.handle) };

        if rc == 0 {
            Err(Error::UnlockFailed)
        } else {
            Ok(())
        }
    }
}

impl Drop for RawNamedLock {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}
