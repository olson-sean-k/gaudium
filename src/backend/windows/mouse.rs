use arrayvec::ArrayVec;
use std::mem;
use winapi::shared::{minwindef, windef};
use winapi::um::winuser;

use backend::IntoRawHandle;
use display::IntoLogical;
use event::{ElementState, InputEvent, ModifierState, MouseButton, MouseMovement};

pub fn parse_raw_input<H>(
    window: H,
    input: &winuser::RAWMOUSE,
) -> Result<impl IntoIterator<Item = InputEvent>, ()>
where
    H: IntoRawHandle<windef::HWND>,
{
    let modifier = ModifierState {}; // TODO: Read modifiers.
    let mut events = ArrayVec::<[InputEvent; 8]>::new();
    // Only emit movement events if there are no button transitions.
    if input.usButtonFlags == 0 {
        unsafe {
            let mut point = mem::uninitialized();
            events.push(InputEvent::MouseMoved {
                movement: MouseMovement {
                    absolute: if winuser::GetCursorPos(&mut point) != 0 {
                        let dpi = 1.0; // TODO: Get the DPI factor.
                        Some((point.x as i32, point.y as i32).into_logical(dpi))
                    }
                    else {
                        None
                    },
                    // TODO: Examine `input.usFlags`.
                    relative: Some((input.lLastX.into(), input.lLastY.into())),
                },
                modifier,
            });
        }
    }
    else {
        let mut push_if = |mask: minwindef::USHORT, button: MouseButton, state: ElementState| {
            if input.usButtonFlags & mask != 0 {
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
    }
    Ok(events)
}
