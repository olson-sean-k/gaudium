use num::{Integer, Num, One, Zero};
use std::mem;
use std::ops::BitAnd;
use std::os::raw;

// Sub-modules are empty on non-target platforms or when disabled by features.
mod wayland;
mod windows;

#[cfg(all(target_os = "linux", feature = "platform-linux-wayland"))]
pub use self::wayland::*;
#[cfg(all(target_os = "windows", feature = "platform-windows-windows"))]
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

fn has_bitflag<T>(value: T, flag: T) -> bool
where
    T: BitAnd<Output = T> + Integer + Num + One + Zero,
{
    if flag == Zero::zero() {
        value & One::one() == Zero::zero()
    }
    else {
        value & flag != Zero::zero()
    }
}
