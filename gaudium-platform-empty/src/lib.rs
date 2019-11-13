use gaudium_core::platform::{PlatformBinding, Proxy};
use gaudium_core::window::WindowBuilder;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {}

impl PlatformBinding for Binding {
    type EventThread = empty::EventThread;
    type WindowBuilder = empty::WindowBuilder;
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

mod empty {
    use arrayvec::ArrayVec;
    use gaudium_core::platform;
    use gaudium_core::reactor::Reactor;
    use gaudium_core::reactor::ThreadContext;
    use gaudium_core::window::WindowHandle;
    use std::process;

    use crate::Binding;

    pub struct EventThread;

    impl platform::Abort<Binding> for EventThread {
        fn run_and_abort<R>(_: ThreadContext, _: WindowHandle<Binding>, reactor: R) -> !
        where
            R: Reactor<Binding>,
        {
            reactor.abort();
            process::abort()
        }
    }

    #[derive(Debug, Eq, Hash, PartialEq)]
    pub struct Device(usize);

    impl platform::Device for Device {
        type Query = Option<Self>;

        fn connected() -> Self::Query {
            None
        }
    }

    impl platform::Handle for Device {
        type Handle = usize;

        fn handle(&self) -> Self::Handle {
            self.0
        }
    }

    #[derive(Debug, Eq, Hash, PartialEq)]
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

    pub struct WindowBuilder {
        title: String,
    }

    impl WindowBuilder {
        pub fn with_title<T>(mut self, title: T) -> Self
        where
            T: AsRef<str>,
        {
            self.title = title.as_ref().to_owned();
            self
        }
    }

    impl Default for WindowBuilder {
        fn default() -> Self {
            WindowBuilder {
                title: String::default(),
            }
        }
    }

    impl platform::WindowBuilder for WindowBuilder {
        type Window = Window;

        fn build(self, _: &ThreadContext) -> Result<Self::Window, ()> {
            // TODO: All windows will compare and hash as equal.
            Ok(Window(0))
        }
    }

    #[derive(Eq, Hash, PartialEq)]
    pub struct Window(u64);

    impl platform::Handle for Window {
        type Handle = u64;

        fn handle(&self) -> Self::Handle {
            self.0
        }
    }
}
