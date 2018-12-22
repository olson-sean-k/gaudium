use crate::event::Event;

pub mod input;

pub trait React {
    fn react(&mut self, event: &Event);
}
