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
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, StatefulReactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder};
//!
//! EventThread::run_and_abort_with(|context| {
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
//! ```
//!
//! ```rust,no_run
//! use gaudium::platform::{Binding, WindowBuilderExt};
//! use gaudium::prelude::*;
//! use gaudium::reactor::{EventThread, FromContext, Reactor, ThreadContext};
//! use gaudium::window::{Window, WindowBuilder, WindowHandle};
//! use std::sync::mpsc::{self, Sender};
//! use std::thread::{self, JoinHandle};
//!
//! struct TestReactor {
//!     window: Window,
//!     tx: Sender<Event>,
//!     handle: JoinHandle<()>,
//! }
//!
//! impl FromContext<Binding> for TestReactor {
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
//! impl Reactor<Binding> for TestReactor {
//!     fn react(&mut self, _: &ThreadContext, event: Event) -> Reaction {
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
//! EventThread::<TestReactor>::run_and_abort()
//! ```

#![allow(unknown_lints)] // Allow clippy lints.

pub use gaudium_core::framework;

pub mod device {
    use crate::platform::Binding;

    pub use gaudium_core::device::Usage;

    pub type DeviceHandle = gaudium_core::device::DeviceHandle<Binding>;
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
    use crate::platform::Binding;

    pub use gaudium_core::event::{
        ApplicationEvent, ElementState, GameControllerAxis, GameControllerButton, InputEvent,
        KeyCode, ModifierState, MouseButton, MouseMovement, MouseWheelDelta, RelativeMotion,
        ScanCode, WindowCloseState, WindowEvent, WindowPosition,
    };

    pub type Event = gaudium_core::event::Event<Binding>;
}

pub mod platform {
    #[cfg(all(
        not(any(target_os = "linux", target_os = "windows")),
        feature = "build-fail-unsupported"
    ))]
    compile_error!("Platform is not supported.");
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    pub use gaudium_platform_empty::{Binding, WindowBuilderExt};
    // TODO: Import types from the Wayland implementation when it is available.
    #[cfg(target_os = "linux")]
    pub use gaudium_platform_empty::{Binding, WindowBuilderExt};
    #[cfg(target_os = "windows")]
    pub use gaudium_platform_windows::{Binding, WindowBuilderExt};

    pub mod alias {
        use crate::platform::Binding;

        pub type Sink = gaudium_core::platform::alias::Sink<Binding>;
    }
}

pub mod prelude {
    pub use crate::event::*;
    pub use crate::reactor::Reaction;

    pub use Reaction::Abort;
    pub use Reaction::Ready;
    pub use Reaction::Wait;
}

pub mod reactor {
    use crate::platform::Binding;

    pub use gaudium_core::reactor::{FromContext, Reaction, Reactor, ThreadContext};

    pub type EventThread<R> = gaudium_core::reactor::EventThread<Binding, R>;
    pub type StatefulReactor<T, F> = gaudium_core::reactor::StatefulReactor<Binding, T, F>;
}

pub mod window {
    use crate::platform::Binding;

    pub type Window = gaudium_core::window::Window<Binding>;
    pub type WindowBuilder = gaudium_core::window::WindowBuilder<Binding>;
    pub type WindowHandle = gaudium_core::window::WindowHandle<Binding>;
}
