use gaudium_core::platform::Map;
use gaudium_core::{platform, window};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {}

impl platform::PlatformBinding for Binding {
    type EventThread = empty::EventThread;
    type WindowBuilder = empty::WindowBuilder;

    type DeviceHandle = u64;
}

pub trait WindowBuilderExt: Sized {
    fn with_title<T>(self, title: T) -> Self
    where
        T: AsRef<str>;
}

impl WindowBuilderExt for window::WindowBuilder<Binding> {
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

    impl platform::EventThread<Binding> for EventThread {
        type Sink = WindowHandle<Binding>;
    }

    impl platform::Abort<Binding> for EventThread {
        fn run_and_abort<R>(_: ThreadContext, _: Self::Sink, reactor: R) -> !
        where
            R: Reactor<Binding>,
        {
            reactor.abort();
            process::abort()
        }
    }

    // TODO: Expose this type when display queries are implemented.
    #[derive(Eq, Hash, PartialEq)]
    pub struct Display(u64);

    impl platform::Display for Display {
        type Query = ArrayVec<[Display; 1]>;

        fn displays() -> Self::Query {
            ArrayVec::from([Display(0)])
        }
    }

    impl platform::Handle for Display {
        type Handle = u64;

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
