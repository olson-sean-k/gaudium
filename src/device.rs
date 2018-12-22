use crate::backend;

// Only specific types are re-exported from backend code. These types are
// opaque, and user code only moves them between Gaudium APIs.
pub type DeviceHandle = backend::DeviceHandle;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Usage {
    Keyboard,
    Mouse,
    GameController,
}
