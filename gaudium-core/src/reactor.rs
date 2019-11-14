//! Event threads and reactors.
//!
//! Gaudium presents an event-driven API based on a platform's event loop. This
//! module provides fundemental types and traits for interacting with the event
//! loop.
//!
//! # Event Threads
//!
//! An _event thread_ is a thread that executes a platform's event loop, which
//! polls for and dispatches events that are delivered by the platform. This
//! thread is managed by the platform and user code has limited control over
//! its execution.
//!
//! The default mode of operation of an event thread is to take complete
//! control of a thread and then diverge. This typically means that the thread
//! (or process) is aborted when the event loop terminates. Some platforms also
//! support joining, in which the event thread relinquishes control back to the
//! initiating code. These modes are described by the `Abort` and `Join`
//! traits.
//!
//! `EventThread` is used to begin an event thread:
//!
//! ```rust,no_run
//! # extern crate gaudium_core;
//! # extern crate gaudium_platform_empty;
//! #
//! # use gaudium_core::event::*;
//! # use gaudium_core::reactor::{EventThread, Reaction, StatefulReactor, ThreadContext};
//! # use gaudium_core::window::{Window, WindowBuilder};
//! # use gaudium_platform_empty::Binding;
//! #
//! # use Reaction::Abort;
//! # use Reaction::Continue;
//! #
//! # fn main() {
//! // Initiate an event thread and its event loop. This cedes control of the
//! // thread to the platform and the reactor.
//! EventThread::<Binding, _>::run_and_abort_with(|context| {
//!     let window = WindowBuilder::<Binding>::default().build(context).unwrap();
//!     (window.handle(), StatefulReactor::from((
//!         // State.
//!         window,
//!         // Reactor.
//!         |_: &mut Window<Binding>, _: &ThreadContext, event| match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => Continue(()),
//!         },
//!     )))
//! })
//! # }
//! ```
//!
//! The above example uses a `StatefulReactor`, but more substantial programs
//! should provide a reactor type by implementing `FromContext` and `Reactor`.
//! See below.
//!
//! Some operations must execute on the event thread and so require a
//! `ThreadContext`. This type does **not** implement the `Send` and `Sync`
//! traits, and is only exposed to the thread executing the event loop. APIs
//! that must be executed on the event thread require a reference to this type.
//!
//! The event loop executed by an event thread has several abstract phases.
//! Transitions between these phases are modeled by `ApplicationEvent`s and are
//! summarized in the table below:
//!
//! | Phase  | Description                                    | Event(s)  |
//! |--------|------------------------------------------------|-----------|
//! | Flush  | Flushes the event queue.                       | `Flushed` |
//! | Poll   | Queries the reactor and polls the event queue. | n/a       |
//! | Resume | Resumes the event loop.                        | `Resumed` |
//!
//! The exact ordering and details of these phases depends on the platform, but
//! this broadly describes an event loop. The _flush_ phase exhausts the event
//! queue, dispatching any and all pending events. The _poll_ phase queries the
//! reactor for a _poll mode_ and proceeds to poll the event queue using the
//! requested mode. Finally, the _resume_ phase begins an iteration of the
//! event loop. When resuming, the `Resumed` event describes the control flow
//! with respect to the previous poll mode, which can be examined to determine
//! if a `WaitUntil` timed out, etc.
//!
//! # Reactors
//!
//! A _reactor_ is a type that reacts to the events dispatched by an event
//! thread and manages state.
//!
//! When the event thread dispatches an event, it is received by a reactor's
//! `react` function. A reactor may handle the event as necessary and then
//! returns a `Reaction`, which either requests that the event thread
//! `Continue` or `Abort`. In addition to reacting to events, reactors also
//! emit poll modes in the poll phase of the event loop via the `poll`
//! function. This function also returns a `Reaction`, but its `Continue`
//! variant provides a poll mode.
//!
//! To implement a reactor and use it with an `EventThread`, implement the
//! `FromContext` and `Reactor` traits:
//!
//! ```rust,no_run
//! # extern crate gaudium_core;
//! # extern crate gaudium_platform_empty;
//! #
//! # use gaudium_core::event::*;
//! # use gaudium_core::reactor::{
//! #     EventThread, FromContext, Poll, Reaction, Reactor, StatefulReactor, ThreadContext
//! # };
//! # use gaudium_core::window::{Window, WindowBuilder, WindowHandle};
//! # use gaudium_platform_empty::Binding;
//! #
//! # use Poll::Wait;
//! # use Reaction::Abort;
//! # use Reaction::Continue;
//! #
//! # fn main() {
//! struct TestReactor {
//!     window: Window<Binding>,
//! }
//!
//! impl FromContext<Binding> for TestReactor {
//!     fn from_context(context: &ThreadContext) -> (WindowHandle<Binding>, Self) {
//!         let window = WindowBuilder::<Binding>::default().build(context).unwrap();
//!         (window.handle(), TestReactor { window })
//!     }
//! }
//!
//! impl Reactor<Binding> for TestReactor {
//!     fn react(&mut self, _: &ThreadContext, event: Event<Binding>) -> Reaction {
//!         match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => Continue(()),
//!         }
//!     }
//!
//!     fn poll(&mut self, _: &ThreadContext) -> Reaction<Poll> {
//!         Continue(Wait)
//!     }
//!
//!     fn abort(self) {}
//! }
//!
//! EventThread::<Binding, TestReactor>::run_and_abort()
//! # }
//! ```

use std::marker::PhantomData;
use std::time::Instant;

use crate::event::Event;
use crate::platform::{Abort, Join, PlatformBinding};
use crate::window::WindowHandle;

/// `PhantomData` that prevents auto-implementation of `Send` and `Sync`.
type ThreadStatic = PhantomData<*mut isize>;

/// Thread-static context.
///
/// A thread context provides state for its event thread and notably does not
/// implement `Send` nor `Sync`. Code that has access to an instance of a
/// `ThreadContext` is necessarily executing on the event thread.
///
/// A thread context is used to create `Reactor`s and `Window`s, which are both
/// operations that must execute on the event thread.
pub struct ThreadContext {
    phantom: ThreadStatic,
}

/// Poll mode.
///
/// Specifies how an event thread should poll events in the event loop. A poll
/// mode is returned as part of a `Reaction` by a reactor's `poll` function.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Poll {
    /// Resumes immediately.
    ///
    /// This poll mode puts the event thread into a busy loop and typically
    /// consumes more CPU time. This is best suited for applications like
    /// games that react to application events and support a busy reactor.
    ///
    /// Resumes with `Resumption::Poll`.
    Ready,
    /// Blocks until an event occurs.
    ///
    /// This poll mode puts the event thread to sleep until an event can be
    /// dispatched. This is best suited for applications that do not need to
    /// react to application events and support an idle reactor.
    ///
    /// Resumes with `Resumption::Poll`.
    Wait,
    /// Blocks until an event occurs or the given instant is reached.
    ///
    /// If an event occurs before the given instant, then this resumes with
    /// `Resumption::Interrupt` and otherwise resumes with
    /// `Resumption::Timeout`.
    WaitUntil(Instant),
}

impl Default for Poll {
    fn default() -> Self {
        Poll::Ready
    }
}

/// Reaction to an event or poll mode query.
///
/// Reactions control the behavior of event loops. Ignoring the payload,
/// `Reaction` either continues or aborts execution.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Reaction<T = ()> {
    /// Continues execution of an event loop.
    ///
    /// When continuing, a payload is also available. When reacting to events,
    /// the payload is `()` and a reaction may only continue or abort. When
    /// responding to a poll mode query, the payload is `Poll`, which
    /// determines the behavior of the event loop if it resumes. In all cases
    /// it is possible to abort.
    Continue(T),
    /// Stops execution of an event loop.
    ///
    /// When aborting, additional events may be dispatched before the event
    /// loop stops. Once an event thread is in the `Abort` state, it cannot
    /// leave that state.
    ///
    /// When using `run_and_abort`, stopping the event loop causes the event
    /// thread or process to abort. Some platforms also support `run_and_join`,
    /// which causes the event thread to relinquish control back to the
    /// initiating code after the event loop stops.
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
    /// The output of this function causes the event loop to continue or abort.
    fn react(&mut self, context: &ThreadContext, event: Event<P>) -> Reaction;

    /// Gets the poll mode that is used when the event loop next resumes.
    ///
    /// The output of this function causes the event loop to continue with the
    /// given poll mode or abort.
    fn poll(&mut self, context: &ThreadContext) -> Reaction<Poll> {
        Default::default()
    }

    /// Manages state when the event loop exits.
    ///
    /// The event thread calls this function when it stops the event loop.
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

    fn poll(&mut self, _: &ThreadContext) -> Reaction<Poll> {
        Poll::Wait.into()
    }
}

/// Conversion from a thread context into a sink and reactor.
///
/// This trait is typically implemented by reactors. A reactor that implements
/// `FromContext` can be used with `EventThread::run_and_abort` and similar
/// functions.
pub trait FromContext<P>: Sized
where
    P: PlatformBinding,
{
    /// Creates an event sink (window handle) and an instance of `Self`.
    fn from_context(context: &ThreadContext) -> (WindowHandle<P>, Self);
}

pub trait IntoReactor<P, R>
where
    P: PlatformBinding,
    R: Reactor<P>,
{
    fn into_reactor(self) -> (WindowHandle<P>, R);
}

impl<'a, P, R> IntoReactor<P, R> for &'a ThreadContext
where
    P: PlatformBinding,
    R: FromContext<P> + Reactor<P>,
{
    fn into_reactor(self) -> (WindowHandle<P>, R) {
        R::from_context(self)
    }
}

/// A reactor that pairs a function with state.
///
/// Some function types implement `Reactor`, but it is not always possible to
/// move state into those functions (closures). `StatefulReactor` explicitly
/// captures state, even if it remains entirely unused.
///
/// This reactor is created from a tuple of state and a function that reacts to
/// events. This is useful in simple applications, but for most applications it
/// is preferable to implement `FromContext` and `Reactor` instead.
///
/// `StatefulReactor` does nothing when it `abort`s and always uses the `Wait`
/// poll mode.
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


/// Creates a `StatefulReactor` from a tuple of state and a function that
/// reacts to events.
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
/// An event thread executes an event loop that polls and dispatches events.
/// Applications typically have only one event thread and when the event loop
/// exits so does the application.
///
/// Events are dispatched to a `Reactor`, which executes user code within the
/// event thread. The reactor processes each event it receives and determines
/// if the event loop continues or aborts.
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
    /// Starts a divergent event thread that aborts when its event loop
    /// terminates.
    ///
    /// ```rust,no_run
    /// # extern crate gaudium_core;
    /// # extern crate gaudium_platform_empty;
    /// #
    /// # use gaudium_core::event::*;
    /// # use gaudium_core::reactor::{
    /// #     EventThread, FromContext, Poll, Reaction, Reactor, StatefulReactor, ThreadContext
    /// # };
    /// # use gaudium_core::window::{Window, WindowBuilder, WindowHandle};
    /// # use gaudium_platform_empty::Binding;
    /// #
    /// # use Poll::Wait;
    /// # use Reaction::Abort;
    /// # use Reaction::Continue;
    /// #
    /// # fn main() {
    /// struct TestReactor {
    ///     window: Window<Binding>,
    /// }
    ///
    /// impl FromContext<Binding> for TestReactor {
    ///     fn from_context(context: &ThreadContext) -> (WindowHandle<Binding>, Self) {
    ///         let window = WindowBuilder::<Binding>::default().build(context).unwrap();
    ///         (window.handle(), TestReactor { window })
    ///     }
    /// }
    ///
    /// impl Reactor<Binding> for TestReactor {
    ///     fn react(&mut self, _: &ThreadContext, event: Event<Binding>) -> Reaction {
    ///         match event {
    ///             Event::Window {
    ///                 event: WindowEvent::Closed(..),
    ///                 ..
    ///             } => Abort,
    ///             _ => Continue(()),
    ///         }
    ///     }
    /// }
    ///
    /// EventThread::<Binding, TestReactor>::run_and_abort()
    /// # }
    pub fn run_and_abort() -> !
    where
        R: FromContext<P>,
    {
        Self::run_and_abort_with(|context| context.into_reactor())
    }

    /// Starts a divergent event thread that aborts when its event loop
    /// terminates.
    ///
    /// Accepts a function that produces a reactor from a thread context.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # extern crate gaudium_core;
    /// # extern crate gaudium_platform_empty;
    /// #
    /// # use gaudium_core::event::*;
    /// # use gaudium_core::reactor::{EventThread, Reaction, StatefulReactor, ThreadContext};
    /// # use gaudium_core::window::{Window, WindowBuilder};
    /// # use gaudium_platform_empty::Binding;
    /// #
    /// # use Reaction::Abort;
    /// # use Reaction::Continue;
    /// #
    /// # fn main() {
    /// // Initiate an event thread and its event loop. This cedes control of the
    /// // thread to the platform and the reactor.
    /// EventThread::<Binding, _>::run_and_abort_with(|context| {
    ///     let window = WindowBuilder::<Binding>::default().build(context).unwrap();
    ///     (window.handle(), StatefulReactor::from((
    ///         // State.
    ///         window,
    ///         // Reactor.
    ///         |_: &mut Window<Binding>, _: &ThreadContext, event| match event {
    ///             Event::Window {
    ///                 event: WindowEvent::Closed(..),
    ///                 ..
    ///             } => Abort,
    ///             _ => Continue(()),
    ///         },
    ///     )))
    /// })
    /// # }
    /// ```
    pub fn run_and_abort_with<F>(f: F) -> !
    where
        F: 'static + FnOnce(&ThreadContext) -> (WindowHandle<P>, R),
    {
        let context = ThreadContext {
            phantom: PhantomData,
        };
        let (sink, reactor) = f(&context);
        <P::EventThread as Abort<P>>::run_and_abort(context, sink, reactor)
    }

    /// Starts an event thread that returns control to the caller when its
    /// event loop terminates.
    pub fn run_and_join()
    where
        R: FromContext<P>,
        P::EventThread: Join<P>,
    {
        Self::run_and_join_with(|context| context.into_reactor())
    }

    /// Starts an event thread that returns control to the caller when its
    /// event loop terminates.
    ///
    /// Accepts a function that produces a reactor from a thread context.
    pub fn run_and_join_with<F>(f: F)
    where
        F: 'static + FnOnce(&ThreadContext) -> (WindowHandle<P>, R),
        P::EventThread: Join<P>,
    {
        let context = ThreadContext {
            phantom: PhantomData,
        };
        let (sink, reactor) = f(&context);
        <P::EventThread as Join<P>>::run_and_join(context, sink, reactor)
    }
}
