use std::fmt::Debug;
use std::hash::Hash;

use crate::reactor::{Reactor, ThreadContext};
use crate::window::WindowHandle;

pub trait PlatformBinding: 'static + Copy + Clone + Debug + PartialEq + Sized {
    type EventThread: Abort<Self> + EventThread<Self, Sink = WindowHandle<Self>>;
    type WindowBuilder: WindowBuilder;

    // TODO: Should this be exposed directly?
    type DeviceHandle: Copy + Debug + Hash + PartialEq + Sized;
}

pub trait EventThread<P>
where
    P: PlatformBinding,
{
    type Sink;
}

pub trait Abort<P>: EventThread<P>
where
    P: PlatformBinding,
{
    fn run_and_abort<R>(context: ThreadContext, sink: Self::Sink, reactor: R) -> !
    where
        R: Reactor<P>;
}

pub trait Join<P>: EventThread<P>
where
    P: PlatformBinding,
{
    fn run_and_join<R>(context: ThreadContext, sink: Self::Sink, reactor: R)
    where
        R: Reactor<P>;
}

// TODO: Implement display queries.
pub trait Display: Handle + Sized {
    type Query: AsRef<[Self]> + IntoIterator<Item = Self>;

    fn displays() -> Self::Query;
}

pub trait WindowBuilder: Default + Sized {
    type Window: Eq + Handle + Hash + Sized;

    fn build(self, context: &ThreadContext) -> Result<Self::Window, ()>;
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

pub mod alias {
    use super::*;

    pub type Sink<P> = <<P as PlatformBinding>::EventThread as EventThread<P>>::Sink;
    pub type Window<P> = <<P as PlatformBinding>::WindowBuilder as WindowBuilder>::Window;
    pub type WindowHandle<P> = <Window<P> as Handle>::Handle;
}
