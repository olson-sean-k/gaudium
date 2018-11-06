use std::collections::HashSet;
use std::ops::Deref;

use event::{ElementState, Event, InputEvent, KeyCode};
use framework::input::state::{AsRawState, Element, Snapshot, SnapshotState};
use framework::React;

impl Element for KeyCode {
    type State = ElementState;
}

/// Keyboard snapshot.
pub struct KeyboardSnapshot {
    new: KeyboardState,
    old: KeyboardState,
}

impl KeyboardSnapshot {
    pub fn new() -> Self {
        KeyboardSnapshot::default()
    }
}

impl Default for KeyboardSnapshot {
    fn default() -> Self {
        KeyboardSnapshot {
            new: KeyboardState::new(),
            old: KeyboardState::new(),
        }
    }
}

impl Deref for KeyboardSnapshot {
    type Target = KeyboardState;

    fn deref(&self) -> &Self::Target {
        &self.new
    }
}

impl Snapshot for KeyboardSnapshot {
    fn snapshot(&mut self) {
        self.old = self.new.clone();
    }
}

impl SnapshotState for KeyboardSnapshot {
    type State = KeyboardState;

    fn new_state(&self) -> &Self::State {
        &self.new
    }

    fn old_state(&self) -> &Self::State {
        &self.old
    }
}

impl React for KeyboardSnapshot {
    fn react(&mut self, event: &Event) {
        if let Event::Input {
            event: InputEvent::KeyboardKeyChanged { keycode, state, .. },
            ..
        } = *event
        {
            if let Some(keycode) = keycode {
                match state {
                    ElementState::Pressed => {
                        self.new.keys.insert(keycode);
                    }
                    ElementState::Released => {
                        self.new.keys.remove(&keycode);
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct KeyboardState {
    keys: HashSet<KeyCode>,
}

impl KeyboardState {
    fn new() -> Self {
        KeyboardState {
            keys: HashSet::new(),
        }
    }
}

impl AsRawState<KeyCode> for KeyboardState {
    type Target = HashSet<KeyCode>;

    fn as_raw_state(&self) -> &Self::Target {
        &self.keys
    }
}
