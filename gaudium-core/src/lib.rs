//! Cross-platform display and input abstraction.
//!
//! # Examples
//!
//! ```rust,no_run
//! # extern crate gaudium_core;
//! # extern crate gaudium_platform_empty;
//! #
//! use gaudium_core::prelude::*;
//! use gaudium_core::reactor::{EventThread, StatefulReactor, ThreadContext};
//! use gaudium_core::window::{Window, WindowBuilder};
//! use gaudium_platform_empty::Platform;
//!
//! # fn main() {
//! EventThread::<Platform, _>::run_and_abort_with(|context| {
//!     let window = WindowBuilder::<Platform>::default().build(context).unwrap();
//!     (window.handle(), StatefulReactor::from((
//!         window,
//!         |_: &mut Window<Platform>, _: &ThreadContext, event| match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => Wait,
//!         },
//!     )))
//! })
//! # }
//! ```
//!
//! ```rust,no_run
//! # extern crate gaudium_platform_empty;
//! # extern crate gaudium_core;
//! #
//! use std::sync::mpsc::{self, Sender};
//! use std::thread::{self, JoinHandle};
//!
//! use gaudium_core::platform::alias::Sink;
//! use gaudium_core::prelude::*;
//! use gaudium_core::reactor::{EventThread, FromContext, Reactor, ThreadContext};
//! use gaudium_core::window::{Window, WindowBuilder};
//! use gaudium_platform_empty::{Platform, WindowBuilderExt};
//!
//! # fn main() {
//! struct TestReactor {
//!     window: Window<Platform>,
//!     tx: Sender<Event<Platform>>,
//!     handle: JoinHandle<()>,
//! }
//!
//! impl FromContext<Platform> for TestReactor {
//!     fn from_context(context: &ThreadContext) -> (Sink<Platform>, Self) {
//!         let window = WindowBuilder::<Platform>::default()
//!             .with_title("Gaudium")
//!             .build(context)
//!             .expect("");
//!         let (tx, rx) = mpsc::channel();
//!         let handle = thread::spawn(move || {
//!             while let Ok(event) = rx.recv() {
//!                 println!("{:?}", event);
//!             }
//!         });
//!         (window.handle(), TestReactor { window, tx, handle })
//!     }
//! }
//!
//! impl Reactor<Platform> for TestReactor {
//!     fn react(&mut self, _: &ThreadContext, event: Event<Platform>) -> Reaction {
//!         match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => {
//!                 if let Some(event) = event.into_remote_event() {
//!                     self.tx.send(event).map(|_| Wait).into()
//!                 }
//!                 else {
//!                     Wait
//!                 }
//!             }
//!         }
//!     }
//!
//!     fn abort(self) {
//!         let TestReactor { tx, handle, .. } = self;
//!         drop(tx);
//!         let _ = handle.join();
//!     }
//! }
//!
//! EventThread::<Platform, TestReactor>::run_and_abort()
//! # }
//! ```

#![allow(unknown_lints)] // Allow clippy lints.

pub mod device;
pub mod display;
pub mod event;
pub mod framework;
pub mod platform;
pub mod reactor;
pub mod window;

pub mod prelude {
    pub use crate::event::*;
    pub use crate::reactor::Reaction;
    pub use crate::reactor::Reaction::Abort;
    pub use crate::reactor::Reaction::Ready;
    pub use crate::reactor::Reaction::Wait;
}

pub trait FromRawHandle<T> {
    fn from_raw_handle(handle: T) -> Self;
}

pub trait IntoRawHandle<T> {
    fn into_raw_handle(self) -> T;
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Sender};
    use std::thread::{self, JoinHandle};

    use crate::platform::alias::*;
    use crate::platform::Platform;
    use crate::prelude::*;
    use crate::reactor::{FromContext, Reactor, ThreadContext};
    use crate::window::{Window, WindowBuilder};

    // For sanity.
    #[test]
    fn test() {
        struct TestReactor<P>
        where
            P: Platform,
        {
            #[allow(dead_code)]
            window: Window<P>,
            tx: Sender<Event<P>>,
            handle: JoinHandle<()>,
        }

        impl<P> FromContext<P> for TestReactor<P>
        where
            P: Platform,
        {
            fn from_context(context: &ThreadContext) -> (Sink<P>, Self) {
                let window = WindowBuilder::<P>::default().build(context).expect("");
                let (tx, rx) = mpsc::channel();
                let handle = thread::spawn(move || {
                    while let Ok(event) = rx.recv() {
                        println!("{:?}", event);
                    }
                });
                (window.handle(), TestReactor { window, tx, handle })
            }
        }

        impl<P> Reactor<P> for TestReactor<P>
        where
            P: Platform,
        {
            fn react(&mut self, _: &ThreadContext, event: Event<P>) -> Reaction {
                match event {
                    Event::Window {
                        event: WindowEvent::Closed(..),
                        ..
                    } => Abort,
                    _ => {
                        if let Some(event) = event.into_remote_event() {
                            self.tx.send(event).map(|_| Wait).into()
                        }
                        else {
                            Wait
                        }
                    }
                }
            }

            fn abort(self) {
                let TestReactor { tx, handle, .. } = self;
                drop(tx);
                let _ = handle.join();
            }
        }
    }
}
