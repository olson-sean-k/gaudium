//! Cross-platform display and input abstraction.
//!
//! Provides a facade over the _core_ and _platform_ crates in the Gaudium
//! ecosystem. This crate selects a suitable platform implementation based on
//! the build target and re-exports core types using bindings to that platform.
//!
//! **This crate requires nightly Rust** for the `type_alias_enum_variant`
//! feature if not used together with the `gaudium-core` crate. For example,
//! the variants of the `Event` alias cannot be used without this feature
//! enabled. If nightly Rust is unacceptable, then the concerned types must be
//! imported from `gaudium-core` when accessing variants.
//!
//! # Examples
//!
//! ```rust,no_run
//! # extern crate gaudium;
//! # extern crate gaudium_core;
//! #
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, StatefulReactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder};
//!
//! # fn main() {
//! EventThread::run_with(|context| {
//!     use gaudium_core::event::Event; // Required to use variants on stable Rust.
//!     let window = WindowBuilder::default().build(context).unwrap();
//!     (window.handle(), StatefulReactor::from((
//!         window,
//!         |_: &mut Window, _: &ThreadContext, event| match event {
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
//! # extern crate gaudium;
//! # extern crate gaudium_core;
//! #
//! use gaudium::platform::{Platform, WindowBuilderExt};
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, FromContext, Reactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder, WindowHandle};
//! use std::sync::mpsc::{self, Sender};
//! use std::thread::{self, JoinHandle};
//!
//! # fn main() {
//! struct TestReactor {
//!     window: Window,
//!     tx: Sender<Event>,
//!     handle: JoinHandle<()>,
//! }
//!
//! impl FromContext<Platform> for TestReactor {
//!     fn from_context(context: &ThreadContext) -> (WindowHandle, Self) {
//!         let window = WindowBuilder::default()
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
//!     fn react(&mut self, _: &ThreadContext, event: Event) -> Poll {
//!         use gaudium_core::event::Event; // Required to use variants on stable Rust.
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
//! # }
//! ```

#![allow(unknown_lints)] // Allow clippy lints.

pub use gaudium_core::framework;

pub mod device {
    use crate::platform::Platform;

    pub use gaudium_core::device::Usage;

    pub type DeviceHandle = gaudium_core::device::DeviceHandle<Platform>;
}

pub mod display {
    pub use gaudium_core::display::{
        FromLogical, FromPhysical, IntoLogical, IntoPhysical, LogicalUnit, PhysicalUnit,
    };

    // TODO: This type will be parameterized by platform.
    //
    //   pub type DisplayHandle = gaudium_core::display::DisplayHandle<Platform>;
    pub use gaudium_core::display::DisplayHandle;
}

pub mod event {
    use crate::platform::Platform;

    pub use gaudium_core::event::{
        ApplicationEvent, ElementState, GameControllerAxis, GameControllerButton, InputEvent,
        KeyCode, ModifierState, MouseButton, MouseMovement, MouseWheelDelta, RelativeMotion,
        ScanCode, WindowCloseState, WindowEvent, WindowPosition,
    };

    pub type Event = gaudium_core::event::Event<Platform>;
}

pub mod platform {
    #[cfg(all(
        not(any(target_os = "linux", target_os = "windows")),
        feature = "build-fail-unsupported"
    ))]
    compile_error!("Platform is not supported.");
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    pub use gaudium_platform_empty::{Platform, WindowBuilderExt};
    // TODO: Import types from the Wayland implementation when it is available.
    #[cfg(target_os = "linux")]
    pub use gaudium_platform_empty::{Platform, WindowBuilderExt};
    #[cfg(target_os = "windows")]
    pub use gaudium_platform_windows::{Platform, WindowBuilderExt};

    pub mod alias {
        use crate::platform::Platform;

        pub type Sink = gaudium_core::platform::alias::Sink<Platform>;
    }
}

pub mod prelude {
    pub use crate::event::*;
    pub use crate::reactor::Poll;

    pub use Poll::Abort;
    pub use Poll::Wait;
}

pub mod reactor {
    use crate::platform::Platform;

    pub use gaudium_core::reactor::{FromContext, Poll, Reactor, ThreadContext};

    pub type EventThread<R> = gaudium_core::reactor::EventThread<Platform, R>;
    pub type StatefulReactor<T, F> = gaudium_core::reactor::StatefulReactor<Platform, T, F>;
}

pub mod window {
    use crate::platform::Platform;

    pub type Window = gaudium_core::window::Window<Platform>;
    pub type WindowBuilder = gaudium_core::window::WindowBuilder<Platform>;
    pub type WindowHandle = gaudium_core::window::WindowHandle<Platform>;
}
