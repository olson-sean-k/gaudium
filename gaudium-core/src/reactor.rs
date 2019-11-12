use std::marker::PhantomData;
use std::time::Instant;

use crate::event::Event;
use crate::platform::alias::*;
use crate::platform::{Abort, Join, PlatformBinding};

/// Event thread context.
///
/// A thread context is an opaque type that provides state for event thread
/// APIs. Notably, it does not implement `Send` or `Sync`. Code that has access
/// to an instance of a `ThreadContext` is necessarily executing on the event
/// thread.
///
/// A thread context is used to create `Reactor`s and `Windows`, which must
/// execute code on the event thread.
pub struct ThreadContext {
    phantom: ThreadStatic,
}

/// `PhantomData` that prevents auto-implementation of `Send` and `Sync`.
pub type ThreadStatic = PhantomData<*mut isize>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Poll {
    Ready,
    Wait,
    WaitUntil(Instant),
}

impl Default for Poll {
    fn default() -> Self {
        Poll::Ready
    }
}

/// Reaction to an event.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Reaction<T = ()> {
    Continue(T),
    /// Stop the event thread and end the process.
    ///
    /// If a reactor aborts, it may still receive additional events before the
    /// event thread stops and the process ends.
    Abort,
}

impl<T> Reaction<T> {
    pub fn map<U, F>(self, mut f: F) -> Reaction<U>
    where
        F: FnMut(T) -> U,
    {
        match self {
            Reaction::Continue(value) => Reaction::Continue(f(value)),
            _ => Reaction::Abort,
        }
    }
}

impl<T> Default for Reaction<T>
where
    T: Default,
{
    fn default() -> Self {
        Reaction::Continue(Default::default())
    }
}

impl From<Poll> for Reaction<Poll> {
    fn from(poll: Poll) -> Self {
        Reaction::Continue(poll)
    }
}

impl<T> From<Option<Reaction<T>>> for Reaction<T> {
    fn from(option: Option<Reaction<T>>) -> Self {
        match option {
            Some(reaction) => reaction,
            None => Reaction::Abort,
        }
    }
}

impl<T, E> From<Result<Reaction<T>, E>> for Reaction<T> {
    fn from(result: Result<Reaction<T>, E>) -> Self {
        match result {
            Ok(reaction) => reaction,
            Err(_) => Reaction::Abort,
        }
    }
}

/// Event thread reactor.
///
/// Reacts to events and controls the poll mode of its event thread. Provides
/// all user state within an event thread.
pub trait Reactor<P>: Sized
where
    P: PlatformBinding,
{
    /// Reacts to an event.
    ///
    /// Must return a `Reaction`, which determines how the event thread
    /// responds.  To terminate the event thread, `Reaction::Abort` should be
    /// returned.
    fn react(&mut self, context: &ThreadContext, event: Event<P>) -> Reaction;

    fn poll(&mut self, context: &ThreadContext) -> Reaction<Poll> {
        Default::default()
    }

    /// Stops the reactor.
    ///
    /// The event thread calls this function when it stops (sometime after
    /// receiving `Reaction::Abort` from `react`).
    fn abort(self) {}
}

impl<P, F> Reactor<P> for F
where
    P: PlatformBinding,
    F: 'static + FnMut(&ThreadContext, Event<P>) -> Reaction,
{
    fn react(&mut self, context: &ThreadContext, event: Event<P>) -> Reaction {
        (self)(context, event)
    }
}

/// Conversion from a thread context.
///
/// This trait is typically implemented by reactors. A reactor that implements
/// `FromContext` can be used by `EventThread::run`.
pub trait FromContext<P>: Sized
where
    P: PlatformBinding,
{
    fn from_context(context: &ThreadContext) -> (Sink<P>, Self);
}

pub trait IntoReactor<P, R>
where
    P: PlatformBinding,
    R: Reactor<P>,
{
    fn into_reactor(self) -> (Sink<P>, R);
}

impl<'a, P, R> IntoReactor<P, R> for &'a ThreadContext
where
    P: PlatformBinding,
    R: FromContext<P> + Reactor<P>,
{
    fn into_reactor(self) -> (Sink<P>, R) {
        R::from_context(self)
    }
}

/// A reactor that pairs a function with state.
///
/// This reactor is created from a tuple of state and a function that reacts to
/// events. This is useful in simple or small applications. For most
/// applications, it is preferable to implement `Reactor` instead.
pub struct StatefulReactor<P, T, F>
where
    P: PlatformBinding,
    F: 'static + FnMut(&mut T, &ThreadContext, Event<P>) -> Reaction,
{
    state: T,
    f: F,
    phantom: PhantomData<P>,
}

impl<P, T, F> Reactor<P> for StatefulReactor<P, T, F>
where
    P: PlatformBinding,
    F: 'static + FnMut(&mut T, &ThreadContext, Event<P>) -> Reaction,
{
    fn react(&mut self, context: &ThreadContext, event: Event<P>) -> Reaction {
        (self.f)(&mut self.state, context, event)
    }

    fn poll(&mut self, context: &ThreadContext) -> Reaction<Poll> {
        Poll::Wait.into()
    }
}

impl<P, T, F> From<(T, F)> for StatefulReactor<P, T, F>
where
    P: PlatformBinding,
    F: 'static + FnMut(&mut T, &ThreadContext, Event<P>) -> Reaction,
{
    fn from(stateful: (T, F)) -> Self {
        let (state, f) = stateful;
        StatefulReactor {
            state,
            f,
            phantom: PhantomData,
        }
    }
}

/// Event thread.
///
/// An event thread continuously polls and dispatches events. When the thread
/// stops, the process stops. Applications typically have only one event
/// thread.
///
/// Events are dispatched to a `Reactor`, which executes user code within the
/// event thread. The reactor processes each event it receives and determines
/// how the next event is polled and dispatched.
///
/// `EventThread` takes control of the thread on which it is started.
pub struct EventThread<P, R>
where
    P: PlatformBinding,
    R: Reactor<P>,
{
    phantom: PhantomData<(P, R)>,
}

impl<P, R> EventThread<P, R>
where
    P: PlatformBinding,
    R: Reactor<P>,
{
    /// Starts an event thread.
    pub fn run_and_abort() -> !
    where
        R: FromContext<P>,
    {
        Self::run_and_abort_with(|context| context.into_reactor())
    }

    /// Starts an event thread with a reactor created with the given function.
    ///
    /// The function accepts a thread context that can be used to create the
    /// reactor and thread-dependent state, such as `Window`s.
    pub fn run_and_abort_with<F>(f: F) -> !
    where
        F: 'static + FnOnce(&ThreadContext) -> (Sink<P>, R),
    {
        let context = ThreadContext {
            phantom: PhantomData,
        };
        let (sink, reactor) = f(&context);
        <P::EventThread as Abort<P>>::run_and_abort(context, sink, reactor)
    }

    pub fn run_and_join()
    where
        R: FromContext<P>,
        P::EventThread: Join<P>,
    {
        Self::run_and_join_with(|context| context.into_reactor())
    }

    pub fn run_and_join_with<F>(f: F)
    where
        F: 'static + FnOnce(&ThreadContext) -> (Sink<P>, R),
        P::EventThread: Join<P>,
    {
        let context = ThreadContext {
            phantom: PhantomData,
        };
        let (sink, reactor) = f(&context);
        <P::EventThread as Join<P>>::run_and_join(context, sink, reactor)
    }
}
