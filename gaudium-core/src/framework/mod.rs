use crate::event::Event;
use crate::platform::Platform;

// TODO: Rework types and traits around `Platform`.
//pub mod input;

pub trait React<P>
where
    P: Platform,
{
    fn react(&mut self, event: &Event<P>);
}
