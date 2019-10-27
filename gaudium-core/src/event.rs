use crate::device::{DeviceHandle, Usage};
use crate::display::{LogicalUnit, PhysicalUnit};
use crate::platform::PlatformBinding;
use crate::window::WindowHandle;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event<P>
where
    P: PlatformBinding,
{
    Application {
        event: ApplicationEvent,
    },
    Input {
        device: DeviceHandle<P>,
        window: Option<WindowHandle<P>>,
        event: InputEvent,
    },
    Window {
        window: WindowHandle<P>,
        event: WindowEvent,
    },
}

impl<P> Event<P>
where
    P: PlatformBinding,
{
    pub fn into_window_event(self, window: WindowHandle<P>) -> Option<Self> {
        let target = Some(window);
        if match self {
            Event::Input { window, .. } => match window {
                Some(window) => Some(window),
                None => target, // Do not filter non-windowed input events.
            },
            Event::Window { window, .. } => Some(window),
            _ => None,
        } == target
        {
            Some(self)
        }
        else {
            None
        }
    }

    pub fn into_device_event(self, device: DeviceHandle<P>) -> Option<Self> {
        let target = device;
        match self {
            Event::Input { device, .. } if target == device => Some(self),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApplicationEvent {
    QueueExhausted, // `Reaction::Ready`.
    TimeoutExpired, // `Reaction::Timeout`.
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    Connected {
        usage: Option<Usage>,
    },
    Disconnected,
    GameControllerButtonChanged {
        button: GameControllerButton,
        state: ElementState,
    },
    // TODO: Is it possible to differentiate axes, throttles, and other value
    //       inputs?
    GameControllerAxisChanged {
        axis: GameControllerAxis,
        value: f64,
    },
    KeyboardKeyChanged {
        scancode: ScanCode,
        keycode: Option<KeyCode>,
        state: ElementState,
        modifier: ModifierState,
    },
    MouseButtonChanged {
        button: MouseButton,
        state: ElementState,
        modifier: ModifierState,
    },
    MouseWheelRotated {
        delta: MouseWheelDelta,
        modifier: ModifierState,
    },
    MouseMoved {
        movement: MouseMovement,
        modifier: ModifierState,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowEvent {
    Closed(WindowCloseState),
    Activated,
    Deactivated,
    Moved(i32, i32),
    Resized(u32, u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowCloseState {
    Requested,
    Committed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

pub type ScanCode = u32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyCode {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModifierState {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Center,
    Other(u8),
}

pub type WindowPosition = (LogicalUnit, LogicalUnit);
pub type RelativeMotion = (PhysicalUnit, PhysicalUnit);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseMovement {
    pub absolute: Option<WindowPosition>,
    pub relative: Option<RelativeMotion>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseWheelDelta {
    Rotational(f64, f64),
    Positional(LogicalUnit, LogicalUnit),
}

pub type GameControllerAxis = u8;

pub type GameControllerButton = u8;
