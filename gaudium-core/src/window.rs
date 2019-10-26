use crate::platform::{alias, Handle, PlatformBinding, Proxy};
use crate::reactor::ThreadContext;
use crate::{FromRawHandle, IntoRawHandle};

/// An opaque type that identifies a `Window`.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct WindowHandle<P>(alias::WindowHandle<P>)
where
    P: PlatformBinding;

impl<P> FromRawHandle<alias::WindowHandle<P>> for WindowHandle<P>
where
    P: PlatformBinding,
{
    fn from_raw_handle(handle: alias::WindowHandle<P>) -> Self {
        WindowHandle(handle)
    }
}

impl<P> IntoRawHandle<alias::WindowHandle<P>> for WindowHandle<P>
where
    P: PlatformBinding,
{
    fn into_raw_handle(self) -> alias::WindowHandle<P> {
        self.0
    }
}

unsafe impl<P> Send for WindowHandle<P> where P: PlatformBinding {}
unsafe impl<P> Sync for WindowHandle<P> where P: PlatformBinding {}

/// Configures and builds a `Window`.
///
/// A `WindowBuilder` is used to create `Window`s. It provides a default
/// configuration that can be customized using a fluent interface.
///
/// By default, `WindowBuilder` only exposes very basic configuration. For more
/// functionality, see the `WindowBuilderExt` extension traits in the
/// `platform` module.
pub struct WindowBuilder<'a, P>
where
    P: PlatformBinding,
{
    inner: <P as PlatformBinding>::WindowBuilder,
    context: &'a ThreadContext,
}

impl<'a, P> WindowBuilder<'a, P>
where
    P: PlatformBinding,
{
    pub fn build(self) -> Result<Window<P>, ()> {
        Window::new(self)
    }
}

impl<'a, P> From<&'a ThreadContext> for WindowBuilder<'a, P>
where
    P: PlatformBinding,
{
    fn from(context: &'a ThreadContext) -> Self {
        WindowBuilder {
            inner: Default::default(),
            context,
        }
    }
}

impl<'a, P> Proxy for WindowBuilder<'a, P>
where
    P: PlatformBinding,
{
    type Inner = P::WindowBuilder;

    fn as_inner(&self) -> &Self::Inner {
        &self.inner
    }

    fn as_inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.inner
    }

    fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(Self::Inner) -> Self::Inner,
    {
        let WindowBuilder { inner, context } = self;
        WindowBuilder {
            inner: f(inner),
            context,
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
pub struct Window<P>
where
    P: PlatformBinding,
{
    inner: alias::Window<P>,
}

impl<P> Window<P>
where
    P: PlatformBinding,
{
    fn new(builder: WindowBuilder<P>) -> Result<Self, ()> {
        use crate::platform::WindowBuilder as _;

        let WindowBuilder {
            inner: builder,
            context,
        } = builder;
        let window = Window {
            inner: builder.build(context)?,
        };
        Ok(window)
    }

    /// Gets the handle of the window.
    pub fn handle(&self) -> WindowHandle<P> {
        WindowHandle(self.inner.handle())
    }

    /// Gets the raw handle of the window used by the platform.
    pub fn raw_handle(&self) -> alias::WindowHandle<P> {
        self.inner.handle()
    }
}
