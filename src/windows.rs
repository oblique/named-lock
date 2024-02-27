use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{
            CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::Threading::{
            CreateMutexW, ReleaseMutex, WaitForSingleObject, INFINITE,
        },
    },
};

use crate::error::*;

#[derive(Debug)]
pub(crate) struct RawNamedLock {
    handle: HANDLE,
}

unsafe impl Sync for RawNamedLock {}
unsafe impl Send for RawNamedLock {}

impl RawNamedLock {
    pub(crate) fn create(name: &str) -> Result<RawNamedLock> {
        let handle = unsafe {
            CreateMutexW(None, false, &HSTRING::from(name))
                .map_err(|e| Error::CreateFailed(std::io::Error::from(e)))?
        };

        Ok(RawNamedLock {
            handle,
        })
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
        unsafe { ReleaseMutex(self.handle).map_err(|_| Error::UnlockFailed) }
    }
}

impl Drop for RawNamedLock {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
