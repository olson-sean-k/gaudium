#![cfg(target_os = "windows")]

use winapi::shared::{ntdef, windef};

use crate::display::LogicalUnit;
use crate::reactor::ThreadContext;
use crate::window::{Window, WindowBuilder};

pub trait WindowExt {
    fn insert(&mut self, builder: WindowBuilder, context: &ThreadContext) -> Result<(), ()>;
    fn transform<T>(&self, position: (T, T)) -> Result<(LogicalUnit, LogicalUnit), ()>
    where
        T: Into<LogicalUnit>;
    fn class_name(&self) -> &[ntdef::WCHAR];
    fn raw_handle(&self) -> windef::HWND;
}

impl WindowExt for Window {
    fn insert(&mut self, builder: WindowBuilder, context: &ThreadContext) -> Result<(), ()> {
        self.as_inner_mut().insert(builder.into_inner(), context)
    }

    fn transform<T>(&self, position: (T, T)) -> Result<(LogicalUnit, LogicalUnit), ()>
    where
        T: Into<LogicalUnit>,
    {
        self.as_inner().transform(position)
    }

    fn class_name(&self) -> &[ntdef::WCHAR] {
        self.as_inner().class_name()
    }

    fn raw_handle(&self) -> windef::HWND {
        self.as_inner().raw_handle()
    }
}

pub trait WindowBuilderExt: Sized {}

impl WindowBuilderExt for WindowBuilder {}
