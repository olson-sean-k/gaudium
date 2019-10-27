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
//! use gaudium_platform_empty::Binding;
//!
//! # fn main() {
//! EventThread::<Binding, _>::run_and_abort_with(|context| {
//!     let window = WindowBuilder::<Binding>::default().build(context).unwrap();
//!     (window.handle(), StatefulReactor::from((
//!         window,
//!         |_: &mut Window<Binding>, _: &ThreadContext, event| match event {
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
//! use gaudium_platform_empty::{Binding, WindowBuilderExt};
//!
//! # fn main() {
//! struct TestReactor {
//!     window: Window<Binding>,
//!     tx: Sender<Event<Binding>>,
//!     handle: JoinHandle<()>,
//! }
//!
//! impl FromContext<Binding> for TestReactor {
//!     fn from_context(context: &ThreadContext) -> (Sink<Binding>, Self) {
//!         let window = WindowBuilder::<Binding>::default()
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
//! impl Reactor<Binding> for TestReactor {
//!     fn react(&mut self, _: &ThreadContext, event: Event<Binding>) -> Reaction {
//!         match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => self.tx.send(event).map(|_| Wait).into(),
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
//! EventThread::<Binding, TestReactor>::run_and_abort()
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
    use crate::platform::PlatformBinding;
    use crate::prelude::*;
    use crate::reactor::{FromContext, Reactor, ThreadContext};
    use crate::window::{Window, WindowBuilder};

    // For sanity.
    #[test]
    fn test() {
        struct TestReactor<P>
        where
            P: PlatformBinding,
        {
            #[allow(dead_code)]
            window: Window<P>,
            tx: Sender<Event<P>>,
            handle: JoinHandle<()>,
        }

        impl<P> FromContext<P> for TestReactor<P>
        where
            P: PlatformBinding,
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
            P: PlatformBinding,
        {
            fn react(&mut self, _: &ThreadContext, event: Event<P>) -> Reaction {
                match event {
                    Event::Window {
                        event: WindowEvent::Closed(..),
                        ..
                    } => Abort,
                    _ => self.tx.send(event).map(|_| Wait).into(),
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
