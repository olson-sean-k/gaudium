#![cfg(target_os = "windows")]

use arrayvec::ArrayVec;
use std::ffi::OsStr;
use winapi::shared::ntdef;

use backend::{FromRawHandle, IntoRawHandle, RawHandle};

mod input;
mod keyboard;
mod mouse;
mod reactor;
mod window;

pub use self::reactor::*;
pub use self::window::*;

trait WideNullTerminated: Sized {
    fn wide_null_terminated(self) -> Vec<ntdef::WCHAR>;
}

impl<T> WideNullTerminated for T
where
    T: AsRef<OsStr>,
{
    fn wide_null_terminated(self) -> Vec<ntdef::WCHAR> {
        use std::os::windows::ffi::OsStrExt;

        self.as_ref()
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect()
    }
}

impl RawHandle for ntdef::HANDLE {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeviceHandle(ntdef::HANDLE);

impl FromRawHandle<ntdef::HANDLE> for DeviceHandle {
    fn from_raw_handle(device: ntdef::HANDLE) -> Self {
        DeviceHandle(device)
    }
}

impl IntoRawHandle<ntdef::HANDLE> for DeviceHandle {
    fn into_raw_handle(self) -> ntdef::HANDLE {
        self.0
    }
}

unsafe impl Send for DeviceHandle {}
unsafe impl Sync for DeviceHandle {}

// TODO: Arrays now implement `Copy` for arbitrary lengths. Once `ArrayVec`
//       supports this, derive `Copy`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DisplayHandle(ArrayVec<[ntdef::WCHAR; 128]>);

impl DisplayHandle {
    fn as_raw_device_name(&self) -> &[ntdef::WCHAR] {
        &self.0
    }
}
