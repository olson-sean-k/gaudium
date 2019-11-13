use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Deref;

use crate::event::ElementState;
use crate::framework::React;
use crate::platform::PlatformBinding;

/// An atomic state of an input element.
pub trait State: Copy + Eq {
    // TODO: Use a default type (`Self`) here once that feature stabilizes.
    /// Representation of a difference between states.
    type Difference: State;

    /// Gets the transition between new and old states. If no transition has
    /// occurred, returns `None`.
    fn transition(new: Self, old: Self) -> Option<Self> {
        if new == old {
            None
        }
        else {
            Some(new)
        }
    }
}

impl State for bool {
    type Difference = Self;
}

impl State for ElementState {
    type Difference = Self;
}

impl<T> State for (T, T)
where
    T: Copy + Eq,
{
    type Difference = Self;
}

/// An input element, such as a button, key, or position.
pub trait Element: Copy + Sized {
    /// Representation of the state of the element.
    type State: State;
}

pub trait AsRawState<E>
where
    E: Element,
{
    // TODO: Use a default type (`E::State`) here once that feature stabilizes.
    type Target;

    fn as_raw_state(&self) -> &Self::Target;
}

/// Provides the complete state for an input element.
pub trait CompositeState<E>
where
    E: Element,
{
    /// Gets the state of an input element.
    fn state(&self, element: E) -> E::State;
}

// Blanket implementation for `CompositeState` for composite states represented by
// a `HashSet`, such as keys and buttons.
impl<E, T> CompositeState<E> for T
where
    T: AsRawState<E, Target = HashSet<E>>,
    E: Element<State = ElementState> + Eq + Hash,
{
    fn state(&self, element: E) -> E::State {
        if self.as_raw_state().contains(&element) {
            ElementState::Pressed
        }
        else {
            ElementState::Released
        }
    }
}

/// Provides a transition state for an input element.
pub trait SnapshotTransition<P, E>
where
    P: PlatformBinding,
    E: Element,
{
    /// Gets the transition state of an input element.
    fn transition(&self, element: E) -> Option<E::State>;
}

impl<P, E, T> SnapshotTransition<P, E> for T
where
    P: PlatformBinding,
    T: Snapshot<P>,
    T::State: CompositeState<E>,
    E: Element,
{
    fn transition(&self, element: E) -> Option<E::State> {
        E::State::transition(
            self.new_state().state(element),
            self.old_state().state(element),
        )
    }
}

/// Determines the difference in state for an input element.
pub trait SnapshotDifference<P, E>
where
    P: PlatformBinding,
    E: Element,
{
    /// Iterable representation of differences in state.
    type Difference: IntoIterator<Item = (E, <E::State as State>::Difference)>;

    /// Gets the difference in state for an input element.
    fn difference(&self) -> Self::Difference;
}

// Blanket implementation for `SnapshotDifference` for composite states
// represented by a `HashSet`, such as keys and buttons.
impl<P, E, S, T> SnapshotDifference<P, E> for T
where
    P: PlatformBinding,
    T: Snapshot<P>,
    T::State: AsRawState<E, Target = HashSet<E>> + CompositeState<E>,
    E: Element<State = S> + Eq + Hash,
    S: State<Difference = S>,
{
    type Difference = Vec<(E, <E::State as State>::Difference)>;

    fn difference(&self) -> Self::Difference {
        self.new_state()
            .as_raw_state()
            .symmetric_difference(self.old_state().as_raw_state())
            .map(|element| (*element, self.new_state().state(*element)))
            .collect()
    }
}

/// A container of device state with new and old states established by updates
/// and snapshots.
pub trait SnapshotState {
    /// Aggregate state for the input device.
    type State;

    /// Gets the new (live) state.
    fn new_state(&self) -> &Self::State;

    /// Gets the old (snapshot) state.
    fn old_state(&self) -> &Self::State;
}

/// A container of device state that can snapshot and compare states.
pub trait Snapshot<P>:
    Deref<Target = <Self as SnapshotState>::State> + React<P> + SnapshotState
where
    P: PlatformBinding,
{
    /// Snapshots the new (live) state.
    fn snapshot(&mut self);
}
