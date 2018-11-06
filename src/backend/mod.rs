use std::mem;
use std::os::raw;

mod facade;
mod wayland;
mod windows;

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
pub use self::facade::*;
#[cfg(target_os = "linux")]
pub use self::wayland::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;

pub trait RawHandle: Copy {}

pub trait FromRawHandle<T>
where
    T: RawHandle,
{
    fn from_raw_handle(handle: T) -> Self;
}

pub trait IntoHandle<T> {
    fn into_handle(self) -> T;
}

impl<T, U> IntoHandle<T> for U
where
    T: FromRawHandle<U>,
    U: RawHandle,
{
    fn into_handle(self) -> T {
        T::from_raw_handle(self)
    }
}

pub trait IntoRawHandle<T>
where
    T: RawHandle,
{
    fn into_raw_handle(self) -> T;
}

impl<T> IntoRawHandle<T> for T
where
    T: RawHandle,
{
    fn into_raw_handle(self) -> T {
        self
    }
}

struct OpaqueBuffer {
    buffer: Vec<u8>,
}

impl OpaqueBuffer {
    pub fn with_size(size: usize) -> Self {
        OpaqueBuffer {
            buffer: Vec::with_capacity(size),
        }
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut raw::c_void {
        mem::transmute(self.buffer.as_mut_ptr())
    }

    pub unsafe fn into_box<T>(self) -> Box<T> {
        let OpaqueBuffer { mut buffer } = self;
        let raw = buffer.as_mut_ptr();
        mem::forget(buffer);
        Box::from_raw(mem::transmute::<_, *mut T>(raw))
    }
}
