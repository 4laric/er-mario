//! Serialize MinHook's *creation and application* across independently linked DLLs.
//! Each DLL has its own MinHook state, but they patch the same DXGI prologues.
//! Creating both trampolines before either patch is applied silently loses one HUD.
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use windows::core::w;

pub struct HudInstallLock(HANDLE);

impl HudInstallLock {
    pub fn acquire() -> Result<Self, String> {
        Self::acquire_timeout(30_000)
    }

    fn acquire_timeout(timeout_ms: u32) -> Result<Self, String> {
        let handle =
            unsafe { CreateMutexW(None, false, w!("Local\\ERArchipelagoHudhookInstall.v1")) }
                .map_err(|e| format!("CreateMutexW: {e:?}"))?;
        let status = unsafe { WaitForSingleObject(handle, timeout_ms) };
        if status == WAIT_OBJECT_0 || status == WAIT_ABANDONED {
            Ok(Self(handle))
        } else {
            unsafe {
                let _ = CloseHandle(handle);
            }
            Err(format!("WaitForSingleObject: {status:?}"))
        }
    }
}

impl Drop for HudInstallLock {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_install_lock_excludes_other_threads_and_releases_on_drop() {
        let first = HudInstallLock::acquire().unwrap();
        assert!(
            std::thread::spawn(|| HudInstallLock::acquire_timeout(25).is_err())
                .join()
                .unwrap()
        );
        drop(first);
        assert!(
            std::thread::spawn(|| HudInstallLock::acquire_timeout(1_000).is_ok())
                .join()
                .unwrap()
        );
    }
}
