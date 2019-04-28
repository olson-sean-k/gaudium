use crate::platform::PlatformBinding;
use crate::{FromRawHandle, IntoRawHandle};

/// An opaque type that identifies an input device.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct DeviceHandle<P>(P::DeviceHandle)
where
    P: PlatformBinding;

impl<P> FromRawHandle<P::DeviceHandle> for DeviceHandle<P>
where
    P: PlatformBinding,
{
    fn from_raw_handle(handle: P::DeviceHandle) -> Self {
        DeviceHandle(handle)
    }
}

impl<P> IntoRawHandle<P::DeviceHandle> for DeviceHandle<P>
where
    P: PlatformBinding,
{
    fn into_raw_handle(self) -> P::DeviceHandle {
        self.0
    }
}

unsafe impl<P> Send for DeviceHandle<P> where P: PlatformBinding {}
unsafe impl<P> Sync for DeviceHandle<P> where P: PlatformBinding {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Usage {
    Keyboard,
    Mouse,
    GameController,
}
