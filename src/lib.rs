//! Cross-platform display and input abstraction.
//!
//! # Examples
//!
//! ```rust ignore,
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, StatefulReactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder};
//!
//! EventThread::run_with_reactor_from(|context| {
//!     let window = WindowBuilder::default().build(context).unwrap();
//!     StatefulReactor::from((
//!         window,
//!         |_: &mut Window, _: &ThreadContext, event| match event {
//!             Event::Window {
//!                 event: WindowEvent::Closed(..),
//!                 ..
//!             } => Abort,
//!             _ => Wait,
//!         },
//!     ))
//! })
//! ```
//!
//! ```rust ignore,
//! use std::sync::mpsc::{self, Sender};
//! use std::thread::{self, JoinHandle};
//!
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, FromContext, Reactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder};
//!
//! struct TestReactor {
//!     window: Window,
//!     tx: Sender<Event>,
//!     handle: JoinHandle<()>,
//! }
//!
//! impl FromContext for TestReactor {
//!     fn from_context(context: &ThreadContext) -> Self {
//!         let window = WindowBuilder::default()
//!             .with_title("Gaudium")
//!             .with_dimensions((480, 320))
//!             .build(context)
//!             .expect("");
//!         let (tx, rx) = mpsc::channel();
//!         let handle = thread::spawn(move || {
//!             while let Ok(event) = rx.recv() {
//!                 println!("{:?}", event);
//!             }
//!         });
//!         TestReactor { window, tx, handle }
//!     }
//! }
//!
//! impl Reactor for TestReactor {
//!     fn react(&mut self, _: &ThreadContext, event: Event) -> Poll {
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
//! EventThread::<TestReactor>::run()
//! ```

#![allow(unknown_lints)] // Allow clippy lints.

extern crate arrayvec;
extern crate fool;
#[macro_use]
extern crate lazy_static;
#[cfg(target_os = "linux")]
extern crate nix;
extern crate num;
#[cfg(target_os = "windows")]
extern crate winapi;

mod backend;
pub mod device;
pub mod display;
pub mod event;
pub mod framework;
pub mod platform;
pub mod reactor;
pub mod window;

pub mod prelude {
    pub use event::*;
    pub use reactor::Poll;
    pub use reactor::Poll::Abort;
    pub use reactor::Poll::Ready;
    pub use reactor::Poll::Timeout;
    pub use reactor::Poll::Wait;
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Sender};
    use std::thread::{self, JoinHandle};

    use platform::windows::*;
    use prelude::*;
    use reactor::{EventThread, FromContext, Reactor, ThreadContext};
    use window::{Window, WindowBuilder};

    // For sanity.
    #[test]
    fn test() {
        struct TestReactor {
            window: Window,
            tx: Sender<Event>,
            handle: JoinHandle<()>,
        }

        impl FromContext for TestReactor {
            fn from_context(context: &ThreadContext) -> Self {
                let window = WindowBuilder::default()
                    .with_title("Gaudium")
                    .with_dimensions((480, 320))
                    .build(context)
                    .expect("");
                let (tx, rx) = mpsc::channel();
                let handle = thread::spawn(move || {
                    while let Ok(event) = rx.recv() {
                        println!("{:?}", event);
                    }
                });
                TestReactor { window, tx, handle }
            }
        }

        impl Reactor for TestReactor {
            fn react(&mut self, _: &ThreadContext, event: Event) -> Poll {
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

        EventThread::<TestReactor>::run()
    }
}
