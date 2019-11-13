#![cfg(target_os = "windows")]

use num::{Integer, Num, One, Zero};
use std::ffi::OsStr;
use std::mem;
use std::ops::BitAnd;
use std::os::raw;
use std::os::windows::ffi::OsStrExt;
use winapi::shared::{minwindef, ntdef};
use winapi::um::processthreadsapi;

mod input;
mod keyboard;
mod mouse;
mod reactor;
mod window;

use gaudium_core::platform::{self, Proxy};
use gaudium_core::window::WindowBuilder;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {}

impl platform::PlatformBinding for Binding {
    type EventThread = reactor::Entry;
    type WindowBuilder = window::WindowBuilder;
    type Device = empty::Device;
    type Display = empty::Display;
}

pub trait WindowBuilderExt: Sized {
    fn with_title<T>(self, title: T) -> Self
    where
        T: AsRef<str>;
}

impl WindowBuilderExt for WindowBuilder<Binding> {
    fn with_title<T>(self, title: T) -> Self
    where
        T: AsRef<str>,
    {
        self.map(move |inner| inner.with_title(title))
    }
}

trait WideNullTerminated: Sized {
    fn wide_null_terminated(self) -> Vec<ntdef::WCHAR>;
}

impl<T> WideNullTerminated for T
where
    T: AsRef<OsStr>,
{
    fn wide_null_terminated(self) -> Vec<ntdef::WCHAR> {
        self.as_ref()
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect()
    }
}

struct OpaqueBuffer {
    buffer: Vec<u8>,
}

impl OpaqueBuffer {
    pub fn with_size(size: usize) -> Self {
        OpaqueBuffer {
            buffer: Vec::with_capacity(size),
        }
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut raw::c_void {
        mem::transmute(self.buffer.as_mut_ptr())
    }

    pub unsafe fn into_box<T>(self) -> Box<T> {
        let OpaqueBuffer { mut buffer } = self;
        let raw = buffer.as_mut_ptr();
        mem::forget(buffer);
        Box::from_raw(mem::transmute::<_, *mut T>(raw))
    }
}

fn has_bit_flags<T>(value: T, flags: T) -> bool
where
    T: BitAnd<Output = T> + Integer + Num + One + Zero,
{
    if flags.is_zero() {
        value & One::one() == Zero::zero()
    }
    else {
        value & flags != Zero::zero()
    }
}

fn exit_process(code: minwindef::UINT) -> ! {
    unsafe {
        processthreadsapi::ExitProcess(code);
    }
    loop {}
}

// TODO: Implement these types.
mod empty {
    use gaudium_core::platform;
    use winapi::shared::ntdef;

    #[derive(Eq, Hash, PartialEq)]
    pub struct Device(ntdef::HANDLE);

    impl platform::Device for Device {
        type Query = Option<Self>;

        fn connected() -> Self::Query {
            None
        }
    }

    impl platform::Handle for Device {
        type Handle = ntdef::HANDLE;

        fn handle(&self) -> Self::Handle {
            self.0
        }
    }

    #[derive(Eq, Hash, PartialEq)]
    pub struct Display(usize);

    impl platform::Display for Display {
        type Query = Option<Self>;

        fn connected() -> Self::Query {
            None
        }
    }

    impl platform::Handle for Display {
        type Handle = usize;

        fn handle(&self) -> Self::Handle {
            self.0
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        use gaudium_core::prelude::*;
        use gaudium_core::reactor::{FromContext, Reactor, ThreadContext};
        use gaudium_core::window::{Window, WindowBuilder, WindowHandle};
        use std::sync::mpsc::{self, Sender};
        use std::thread::{self, JoinHandle};

        use crate::Binding;

        struct TestReactor {
            window: Window<Binding>,
            tx: Sender<Event<Binding>>,
            handle: JoinHandle<()>,
        }

        impl FromContext<Binding> for TestReactor {
            fn from_context(context: &ThreadContext) -> (WindowHandle<Binding>, Self) {
                let window = WindowBuilder::<Binding>::default()
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
            fn react(&mut self, _: &ThreadContext, event: Event<Binding>) -> Reaction {
                match event {
                    Event::Window {
                        event: WindowEvent::Closed(..),
                        ..
                    } => Abort,
                    Event::Application { .. } => Continue(()),
                    _ => self.tx.send(event).map(|_| Continue(())).into(),
                }
            }

            fn abort(self) {
                let TestReactor { tx, handle, .. } = self;
                drop(tx);
                let _ = handle.join();
            }
        }

        use gaudium_core::reactor::EventThread;
        EventThread::<Binding, TestReactor>::run_and_abort()
    }
}
