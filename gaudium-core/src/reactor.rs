use std::marker::PhantomData;

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

/// Reaction to an event.
///
/// Each time a `Reactor` reacts to an event, it must yield `Reaction` to
/// specify the poll mode used by the event thread or to terminate. The poll
/// mode determines how the next event is polled and dispatched.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Reaction {
    /// Dispatch pending events. If there are no pending events, then
    /// `ApplicationEvent::QueueExhausted` is dispatched.
    Ready,
    /// Block the event thread until an event arrives and can be dispatched.
    Wait,
    /// Stop the event thread and end the process.
    ///
    /// If a reactor aborts, it may still receive additional events before the
    /// event thread stops and the process ends.
    Abort,
}

impl Default for Reaction {
    fn default() -> Self {
        Reaction::Wait
    }
}

impl From<Option<Reaction>> for Reaction {
    fn from(option: Option<Reaction>) -> Self {
        match option {
            Some(reaction) => reaction,
            None => Reaction::Abort,
        }
    }
}

impl<E> From<Result<Reaction, E>> for Reaction {
    fn from(result: Result<Reaction, E>) -> Self {
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
