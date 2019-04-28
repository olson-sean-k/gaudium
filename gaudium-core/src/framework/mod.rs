use crate::event::Event;
use crate::platform::PlatformBinding;

// TODO: Rework types and traits around `Platform`.
//pub mod input;

pub trait React<P>
where
    P: PlatformBinding,
{
    fn react(&mut self, event: &Event<P>);
}
