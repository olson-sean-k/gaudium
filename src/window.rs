use crate::backend;
use crate::display::LogicalUnit;
use crate::reactor::ThreadContext;

// Only specific types are re-exported from backend code. These types are
// opaque, and user code only moves them between Gaudium APIs.
/// An opaque type that identifies a `Window`.
pub type WindowHandle = backend::WindowHandle;

/// Configures and builds a `Window`.
///
/// A `WindowBuilder` is used to create `Window`s. It provides a default
/// configuration that can be customized using a fluent interface.
///
/// By default, `WindowBuilder` only exposes very basic configuration. For more
/// functionality, see the `WindowBuilderExt` extension traits in the
/// `platform` module.
pub struct WindowBuilder {
    inner: backend::WindowBuilder,
}

impl WindowBuilder {
    pub fn with_title(mut self, title: &str) -> Self {
        self.inner.with_title(title);
        self
    }

    pub fn with_dimensions<T>(mut self, dimensions: (T, T)) -> Self
    where
        T: Into<LogicalUnit>,
    {
        self.inner.with_dimensions(dimensions);
        self
    }

    pub fn build(self, context: &ThreadContext) -> Result<Window, ()> {
        Window::new(self, context)
    }

    pub(crate) fn into_inner(self) -> backend::WindowBuilder {
        let WindowBuilder { inner } = self;
        inner
    }

    pub(crate) fn as_inner(&self) -> &backend::WindowBuilder {
        &self.inner
    }

    pub(crate) fn as_inner_mut(&mut self) -> &mut backend::WindowBuilder {
        &mut self.inner
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        WindowBuilder {
            inner: backend::WindowBuilder::default(),
        }
    }
}

/// A rendering target and event sink.
///
/// `Window`s manifest differently depending on the target platform, but always
/// provide a rendering target for graphics and an event sink. On desktop
/// platforms, it is typically possible to create multiple windows and these
/// windows can be manipulated by the user. On some platforms, it may not be
/// possible to create more than one window and the window may act as an analog
/// for a single display.
///
/// A `Window` is required in order to receive the complete suite of events in
/// an `EventThread`. On most platforms, input events cannot be received
/// without first creating a `Window`.
///
/// `Window` can be moved across threads but must be created on the event
/// thread using a `WindowBuilder`. When a `Window` is dropped, it is closed.
///
/// Because windows are fairly abstract and manifest differently, `Window`
/// provides very limited functionality. See the `WindowExt` extension traits
/// in the `platform` module for additional per-platform features.
#[derive(Eq, Hash, PartialEq)]
pub struct Window {
    inner: backend::Window,
}

impl Window {
    fn new(builder: WindowBuilder, context: &ThreadContext) -> Result<Self, ()> {
        let window = Window {
            inner: builder.inner.build(context)?,
        };
        Ok(window)
    }

    /// Gets the handle of the window.
    pub fn handle(&self) -> WindowHandle {
        self.inner.handle()
    }

    pub(crate) fn as_inner(&self) -> &backend::Window {
        &self.inner
    }

    pub(crate) fn as_inner_mut(&mut self) -> &mut backend::Window {
        &mut self.inner
    }
}
