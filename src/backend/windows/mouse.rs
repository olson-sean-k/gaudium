use arrayvec::ArrayVec;
use std::mem;
use winapi::shared::{minwindef, ntdef, windef};
use winapi::um::winuser;

use backend::{self, IntoRawHandle};
use display::IntoLogical;
use event::{ElementState, InputEvent, ModifierState, MouseButton, MouseMovement, MouseWheelDelta};

const EVENT_BUFFER_SIZE: usize = 8;

pub fn parse_raw_input<H>(
    _: H,
    input: &winuser::RAWMOUSE,
) -> Result<impl IntoIterator<Item = InputEvent>, ()>
where
    H: IntoRawHandle<windef::HWND>,
{
    let modifier = ModifierState {}; // TODO: Read modifiers.
    let mut events = ArrayVec::<[InputEvent; EVENT_BUFFER_SIZE]>::new();
    if let Ok(event) = parse_movement(input, modifier) {
        events.push(event);
    }
    if let Ok(event) = parse_wheel(input, modifier) {
        events.push(event);
    }
    let _ = parse_buttons_into(input, modifier, &mut events);
    Ok(events)
}

fn parse_movement(input: &winuser::RAWMOUSE, modifier: ModifierState) -> Result<InputEvent, ()> {
    let mut point = unsafe { mem::uninitialized() };
    let event = InputEvent::MouseMoved {
        movement: MouseMovement {
            absolute: if unsafe { winuser::GetCursorPos(&mut point) != 0 } {
                let dpi = 1.0; // TODO: Get the DPI factor.
                Some((point.x as i32, point.y as i32).into_logical(dpi))
            }
            else {
                None
            },
            // The `MOUSE_MOVE_RELATIVE` flag is typically set. If not, then
            // absolute motion events will be queued for each Raw Input event.
            relative: if backend::has_bitflag(input.usFlags, winuser::MOUSE_MOVE_RELATIVE) {
                Some((input.lLastX.into(), input.lLastY.into()))
            }
            else {
                None
            },
        },
        modifier,
    };
    if let Some(event) = match event {
        InputEvent::MouseMoved {
            movement:
                MouseMovement {
                    relative: Some((x, y)),
                    ..
                },
            ..
        } if x != 0.0.into() || y != 0.0.into() => Some(event),
        InputEvent::MouseMoved {
            movement: MouseMovement { relative: None, .. },
            ..
        } => Some(event),
        _ => None,
    } {
        Ok(event)
    }
    else {
        Err(())
    }
}

fn parse_wheel(input: &winuser::RAWMOUSE, modifier: ModifierState) -> Result<InputEvent, ()> {
    if backend::has_bitflag(input.usButtonFlags, winuser::RI_MOUSE_WHEEL) {
        Ok(InputEvent::MouseWheelRotated {
            delta: MouseWheelDelta::Rotational(
                0.0,
                ((input.usButtonData as ntdef::SHORT) / winuser::WHEEL_DELTA) as f64,
            ),
            modifier,
        })
    }
    else {
        Err(())
    }
}

fn parse_buttons_into(
    input: &winuser::RAWMOUSE,
    modifier: ModifierState,
    events: &mut ArrayVec<[InputEvent; EVENT_BUFFER_SIZE]>,
) -> Result<(), ()> {
    let mut push_if = |mask: minwindef::USHORT, button: MouseButton, state: ElementState| {
        if backend::has_bitflag(input.usButtonFlags, mask) {
            events.push(InputEvent::MouseButtonChanged {
                button,
                state,
                modifier,
            });
        }
    };
    // TODO: Read other button states.
    push_if(
        winuser::RI_MOUSE_LEFT_BUTTON_DOWN,
        MouseButton::Left,
        ElementState::Pressed,
    );
    push_if(
        winuser::RI_MOUSE_LEFT_BUTTON_UP,
        MouseButton::Left,
        ElementState::Released,
    );
    push_if(
        winuser::RI_MOUSE_RIGHT_BUTTON_DOWN,
        MouseButton::Right,
        ElementState::Pressed,
    );
    push_if(
        winuser::RI_MOUSE_RIGHT_BUTTON_UP,
        MouseButton::Right,
        ElementState::Released,
    );
    push_if(
        winuser::RI_MOUSE_MIDDLE_BUTTON_DOWN,
        MouseButton::Center,
        ElementState::Pressed,
    );
    push_if(
        winuser::RI_MOUSE_MIDDLE_BUTTON_UP,
        MouseButton::Center,
        ElementState::Released,
    );
    Ok(())
}
