use std::fmt::Debug;
use std::hash::Hash;

use crate::reactor::{Reactor, ThreadContext};
use crate::window;

pub type Window<P> = <<P as PlatformBinding>::WindowBuilder as WindowBuilder>::Window;

pub type DeviceHandle<P> = <<P as PlatformBinding>::Device as Handle>::Handle;
pub type DisplayHandle<P> = <<P as PlatformBinding>::Display as Handle>::Handle;
pub type WindowHandle<P> = <Window<P> as Handle>::Handle;

// TODO: Is it possible to remove these supertraits and bounds?
pub trait PlatformBinding: 'static + Copy + Clone + Debug + PartialEq + Sized {
    type EventThread: Abort<Self>;
    type WindowBuilder: WindowBuilder;
    type Device: Device;
    type Display: Display;
}

pub trait Abort<P>
where
    P: PlatformBinding,
{
    fn run_and_abort<R>(context: ThreadContext, sink: window::WindowHandle<P>, reactor: R) -> !
    where
        R: Reactor<P>;
}

pub trait Join<P>
where
    P: PlatformBinding,
{
    fn run_and_join<R>(context: ThreadContext, sink: window::WindowHandle<P>, reactor: R)
    where
        R: Reactor<P>;
}

pub trait WindowBuilder: Default + Sized {
    type Window: Eq + Handle + Hash + Sized;

    fn build(self, context: &ThreadContext) -> Result<Self::Window, ()>;
}

pub trait Display: Handle + Sized {
    type Query: IntoIterator<Item = Self>;

    fn connected() -> Self::Query;
}

pub trait Device: Handle + Sized {
    type Query: IntoIterator<Item = Self>;

    fn connected() -> Self::Query;
}

pub trait Handle {
    type Handle: Copy + Debug + Hash + PartialEq + Sized;

    fn handle(&self) -> Self::Handle;
}

pub trait Proxy {
    type Inner;

    fn as_inner(&self) -> &Self::Inner;

    fn as_inner_mut(&mut self) -> &mut Self::Inner;

    fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(Self::Inner) -> Self::Inner;
}
