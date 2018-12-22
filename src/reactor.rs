use std::marker::PhantomData;
use std::time::Duration;

use crate::backend;
use crate::event::Event;

// Only specific types are re-exported from backend code. These types are
// opaque, and user code only moves them between Gaudium APIs.
/// Event thread context.
///
/// A thread context is an opaque type that provides state for event thread
/// APIs. Notably, it does not implement `Send` or `Sync`. Code that has access
/// to an instance of a `ThreadContext` is necessarily executing on the event
/// thread.
///
/// A thread context is used to create `Reactor`s and `Windows`, which must
/// execute code on the event thread.
pub type ThreadContext = backend::ThreadContext;

/// `PhantomData` that prevents auto-implementation of `Send` and `Sync`.
pub(crate) type ThreadStatic = PhantomData<*mut isize>;

/// Poll mode.
///
/// Each time a `Reactor` reacts to an event, it must yield `Poll` to specify
/// the poll mode used by the event thread. The poll mode determines how the
/// next event is polled and dispatched.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Poll {
    /// Dispatch pending events. If there are no pending events, then
    /// `ApplicationEvent::QueueExhausted` is dispatched.
    Ready,
    /// Block the event thread until an event arrives and can be dispatched.
    Wait,
    /// Block the event thread until an event arrives and can be dispatched or
    /// the given timeout expires. If the timeout expires, then
    /// `ApplicationEvent::TimeoutExpired` is dispatched.
    Timeout(Duration),
    /// Stop the event thread and end the process.
    ///
    /// If a reactor aborts, it may still receive additional events before the
    /// event thread stops and the process ends.
    Abort,
}

impl Default for Poll {
    fn default() -> Self {
        Poll::Wait
    }
}

impl From<Option<Poll>> for Poll {
    fn from(option: Option<Poll>) -> Self {
        match option {
            Some(poll) => poll,
            None => Poll::Abort,
        }
    }
}

impl<E> From<Result<Poll, E>> for Poll {
    fn from(result: Result<Poll, E>) -> Self {
        match result {
            Ok(poll) => poll,
            Err(_) => Poll::Abort,
        }
    }
}

/// Event thread reactor.
///
/// Reacts to events and controls the poll mode of its event thread. Provides
/// all user state within an event thread.
pub trait Reactor: Sized {
    /// Reacts to an event.
    ///
    /// Must return a poll mode, which determines how the next event is polled
    /// and dispatched. To end the event thread, `Poll::Abort` should be
    /// returned.
    fn react(&mut self, context: &ThreadContext, event: Event) -> Poll;

    /// Stops the reactor.
    ///
    /// The event thread calls this function when it stops (sometime after
    /// receiving `Poll::Abort` from the `react`).
    fn abort(self) {}
}

impl<F> Reactor for F
where
    F: 'static + FnMut(&ThreadContext, Event) -> Poll,
{
    fn react(&mut self, context: &ThreadContext, event: Event) -> Poll {
        (self)(context, event)
    }
}

/// Conversion from a thread context.
///
/// This trait is typically implemented by reactors. A reactor that implements
/// `FromContext` can be used by `EventThread::run`.
pub trait FromContext {
    fn from_context(context: &ThreadContext) -> Self;
}

pub trait IntoReactor<R>
where
    R: Reactor,
{
    fn into_reactor(self) -> R;
}

impl<'a, R> IntoReactor<R> for &'a ThreadContext
where
    R: FromContext + Reactor,
{
    fn into_reactor(self) -> R {
        R::from_context(self)
    }
}

/// A reactor that pairs a function with state.
///
/// This reactor is created from a tuple of state and a function that reacts to
/// events. This is useful in simple or small applications. For most
/// applications, it is preferable to implement `Reactor` instead.
pub struct StatefulReactor<T, F>
where
    F: 'static + FnMut(&mut T, &ThreadContext, Event) -> Poll,
{
    state: T,
    f: F,
}

impl<T, F> Reactor for StatefulReactor<T, F>
where
    F: 'static + FnMut(&mut T, &ThreadContext, Event) -> Poll,
{
    fn react(&mut self, context: &ThreadContext, event: Event) -> Poll {
        (self.f)(&mut self.state, context, event)
    }
}

impl<T, F> From<(T, F)> for StatefulReactor<T, F>
where
    F: 'static + FnMut(&mut T, &ThreadContext, Event) -> Poll,
{
    fn from(stateful: (T, F)) -> Self {
        let (state, f) = stateful;
        StatefulReactor { state, f }
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
pub struct EventThread<R>
where
    R: Reactor,
{
    phantom: PhantomData<R>,
}

impl<R> EventThread<R>
where
    R: Reactor,
{
    /// Starts an event thread.
    pub fn run() -> !
    where
        R: FromContext,
    {
        EventThread::<R>::run_with_reactor_from(|context| context.into_reactor())
    }

    /// Starts an event thread with the given reactor.
    pub fn run_with_reactor<S>(reactor: S) -> !
    where
        S: Into<R>,
    {
        backend::EventThread::run_with_reactor(reactor)
    }

    /// Starts an event thread with a reactor created with the given function.
    ///
    /// The function accepts a thread context that can be used to create the
    /// reactor and thread-dependent state, such as `Window`s.
    pub fn run_with_reactor_from<F>(f: F) -> !
    where
        F: 'static + FnOnce(&ThreadContext) -> R,
    {
        backend::EventThread::run_with_reactor_from(f)
    }
}
