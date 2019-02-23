use gaudium_core::event::{ElementState, InputEvent, ModifierState};
use winapi::shared::minwindef;
use winapi::um::winuser;

pub fn parse_raw_input(input: &winuser::RAWKEYBOARD) -> Result<InputEvent, ()> {
    // TODO: Map the virtual keycode and modifier state.
    Ok(InputEvent::KeyboardKeyChanged {
        scancode: input.MakeCode as u32,
        keycode: None,
        state: if input.Flags & winuser::RI_KEY_BREAK as minwindef::USHORT != 0 {
            ElementState::Released
        }
        else {
            ElementState::Pressed
        },
        modifier: ModifierState {},
    })
}
