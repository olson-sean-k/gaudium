![Gaudium](https://raw.githubusercontent.com/olson-sean-k/gaudium/master/doc/gaudium.png)

**Gaudium** is a Rust library for cross-platform display and input abstraction.

[![Build Status](https://travis-ci.org/olson-sean-k/gaudium.svg?branch=master)](https://travis-ci.org/olson-sean-k/gaudium)
[![Documentation](https://docs.rs/gaudium/badge.svg)](https://doc.rs/gaudium)
[![Crate](https://img.shields.io/crates/v/gaudium.svg)](https://crates.io/crates/gaudium)

## Event Threads and Reactors

An _event thread_ is used to process events dispatched from the target
platform. An event thread manages state and marshals events to a _reactor_,
which allows user code to react to these events. This user code is always
executed on the event thread and typically runs within platform code (within an
OS or process event loop, etc.).

Reactors can immediately handle events within the event thread or further
dispatch events to other threads as needed.

```rust
use gaudium::platform::{Binding, WindowBuilderExt};
use gaudium::prelude::*;
use gaudium::reactor::{EventThread, FromContext, Reactor, ThreadContext};
use gaudium::window::{Window, WindowBuilder, WindowHandle};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

struct TestReactor {
    window: Window,
    tx: Sender<Event>,
    handle: JoinHandle<()>,
}

impl FromContext<Binding> for TestReactor {
    fn from_context(context: &ThreadContext) -> (WindowHandle, Self) {
        let window = WindowBuilder::default()
            .with_title("Gaudium")
            .build(context)
            .expect("");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                println!("{:?}", event);
            }
        });
        (window.handle(), TestReactor { window, tx, handle })
    }
}

impl Reactor<Binding> for TestReactor {
    fn react(&mut self, _: &ThreadContext, event: Event) -> Reaction {
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

EventThread::<TestReactor>::run_and_abort()
```

The above example creates a reactor with a window (see below) and spawns another
thread that prints the events it receives from a channel. The reactor causes the
application to stop when the window is closed and otherwise sends remote events
to the other thread to be printed.

## Input

Gaudium provides input events for keyboards, mice, and game controllers,
including gamepads and joysticks. Gamepads and joysticks are handled in as
generic a fashion as possible, with no symbolic mappings. The `framework` module
(in `gaudium-core` and `gaudium`) provides additional tools for managing state
and creating application-specific mappings for input devices.

## Displays and Windowing

A _window_ is a rendering target and event sink. Conceptually, a window is
presented on a _display_, which is a physical device that presents a window to
the user. On platforms that support desktop environments, a window can be
directly manipulated by users, but on some platforms a window is a thin
abstraction for an entire display and only one window can be created per
process. On some platforms, closing or dropping a window causes the event thread
to abort.

## Platforms and Crates

At this time, Gaudium is very experimental and incomplete. Development is done
exlcusively against the [Windows SDK](https://crates.io/crates/winapi), but
Gaudium abstracts this code and additional platform support is planned.
Anything in the `0.0.*` series is very unstable! Platform support is summarized
in the following table:

| Platform    | Operating Systems | Status      |
|-------------|-------------------|-------------|
| Windows SDK | Windows           | In Progress |
| Wayland     | Linux             | Planned     |
| WASM        | n/a               | Planned     |

Gaudium is comprised of multiple crates. The `gaudium-core` crate provides the
abstraction layer and core constructs. Various `gaudium-platform-*` crates
provide implementations for platforms. Finally, the `gaudium` crate is an
optional facade that automatically chooses a suitable platform implementation
and re-exports types with bindings to that platform and common extension
traits.

Note that _platforms_ do not always map one-to-one to _targets_ or _operating
systems_. Platform crates may be viable on more than one target or operating
system. An implementation is chosen by depending on a viable platform crate and
binding its API with `gaudium-core`.

Platform-specific features are exposed by extension traits in the `platform`
module of `gaudium` or directly from platform crates. For example, by using the
`platform::WindowExt` trait from `gaudium` on Windows, coordinates on a display
can be transformed to a window's local coordinate system and child windows can
be created within a parent window. These extension traits form an implicit API
that is shared across platform crates, so commonly supported operations can be
used without conditional code or compilation.
