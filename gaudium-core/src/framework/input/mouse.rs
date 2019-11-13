use std::collections::HashSet;
use std::ops::Deref;

use crate::event::{ElementState, Event, InputEvent, MouseButton, MouseMovement};
use crate::framework::input::state::{
    AsRawState, CompositeState, Element, Snapshot, SnapshotDifference, SnapshotState,
    SnapshotTransition, State,
};
use crate::framework::React;
use crate::platform::PlatformBinding;

impl Element for MouseButton {
    type State = ElementState;
}

/// Mouse position (pointer) element.
#[derive(Clone, Copy, Debug)]
pub struct MousePosition;

impl Element for MousePosition {
    type State = (i32, i32);
}

/// Mouse proximity element. Indicates whether or not the mouse position
/// (pointer) is within the bounds of the window.
#[derive(Clone, Copy, Debug)]
pub struct MouseProximity;

impl Element for MouseProximity {
    type State = bool;
}

/// Mouse (pointer) snapshot.
pub struct MouseSnapshot {
    new: MouseState,
    old: MouseState,
}

impl MouseSnapshot {
    pub fn new() -> Self {
        MouseSnapshot::default()
    }
}

impl Default for MouseSnapshot {
    fn default() -> Self {
        MouseSnapshot {
            new: MouseState::new(),
            old: MouseState::new(),
        }
    }
}

impl Deref for MouseSnapshot {
    type Target = MouseState;

    fn deref(&self) -> &Self::Target {
        &self.new
    }
}

impl<P> Snapshot<P> for MouseSnapshot
where
    P: PlatformBinding,
{
    fn snapshot(&mut self) {
        self.old = self.new.clone();
    }
}

impl<P> SnapshotDifference<P, MousePosition> for MouseSnapshot
where
    P: PlatformBinding,
{
    type Difference = Option<(
        MousePosition,
        <<MousePosition as Element>::State as State>::Difference,
    )>;

    // This is distinct from `SnapshotTransition::transition`. That function
    // indicates whether or not a change has occurred and yields the current
    // state. This function instead yields a *difference*, for which the type
    // representing the change in state can be entirely different than the type
    // of the state itself. For mouse position, `transition` yields a point and
    // `difference` yields a vector.
    fn difference(&self) -> Self::Difference {
        // TODO: Consider using a more sophisticated type for position state.
        //
        //   let difference = self.new.state(MousePosition) - self.old.state(MousePosition);
        //   (!difference.is_zero()).some((MousePosition, difference))
        None
    }
}

impl<P> SnapshotDifference<P, MouseProximity> for MouseSnapshot
where
    P: PlatformBinding,
{
    type Difference = Option<(
        MouseProximity,
        <<MouseProximity as Element>::State as State>::Difference,
    )>;

    fn difference(&self) -> Self::Difference {
        <Self as SnapshotTransition<P, _>>::transition(self, MouseProximity)
            .map(|state| (MouseProximity, state))
    }
}

impl SnapshotState for MouseSnapshot {
    type State = MouseState;

    fn new_state(&self) -> &Self::State {
        &self.new
    }

    fn old_state(&self) -> &Self::State {
        &self.old
    }
}

impl<P> React<P> for MouseSnapshot
where
    P: PlatformBinding,
{
    fn react(&mut self, event: &Event<P>) {
        match *event {
            Event::Input {
                event: InputEvent::MouseButtonChanged { button, state, .. },
                ..
            } => match state {
                ElementState::Pressed => {
                    self.new.buttons.insert(button);
                }
                ElementState::Released => {
                    self.new.buttons.remove(&button);
                }
            },
            Event::Input {
                event:
                    InputEvent::MouseMoved {
                        movement:
                            MouseMovement {
                                absolute: Some((x, y)),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                // TODO: Reconcile these types.
                self.new.position = (x.into(), y.into());
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct MouseState {
    buttons: HashSet<MouseButton>,
    position: (i32, i32),
    proximity: bool,
}

impl MouseState {
    fn new() -> Self {
        MouseState {
            buttons: HashSet::new(),
            position: (0, 0),
            proximity: false,
        }
    }
}

impl AsRawState<MouseButton> for MouseState {
    type Target = HashSet<MouseButton>;

    fn as_raw_state(&self) -> &Self::Target {
        &self.buttons
    }
}

impl CompositeState<MousePosition> for MouseState {
    fn state(&self, _: MousePosition) -> <MousePosition as Element>::State {
        self.position
    }
}

impl CompositeState<MouseProximity> for MouseState {
    fn state(&self, _: MouseProximity) -> <MouseProximity as Element>::State {
        self.proximity
    }
}
