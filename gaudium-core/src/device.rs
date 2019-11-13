use crate::platform::{self, PlatformBinding};
use crate::{FromRawHandle, IntoRawHandle};

/// An opaque type that identifies an input device.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct DeviceHandle<P>(platform::DeviceHandle<P>)
where
    P: PlatformBinding;

impl<P> FromRawHandle<platform::DeviceHandle<P>> for DeviceHandle<P>
where
    P: PlatformBinding,
{
    fn from_raw_handle(handle: platform::DeviceHandle<P>) -> Self {
        DeviceHandle(handle)
    }
}

impl<P> IntoRawHandle<platform::DeviceHandle<P>> for DeviceHandle<P>
where
    P: PlatformBinding,
{
    fn into_raw_handle(self) -> platform::DeviceHandle<P> {
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
