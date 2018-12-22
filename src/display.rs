use std::ops::Deref;

use crate::backend;

// Only specific types are re-exported from backend code. These types are
// opaque, and user code only moves them between Gaudium APIs.
pub type DisplayHandle = backend::DisplayHandle;

pub trait FromLogical<T> {
    fn from_logical(logical: T, dpi: f64) -> Self;
}

pub trait IntoPhysical<T> {
    fn into_physical(self, dpi: f64) -> T;
}

impl<T, U> IntoPhysical<T> for U
where
    T: FromLogical<U>,
{
    fn into_physical(self, dpi: f64) -> T {
        T::from_logical(self, dpi)
    }
}

pub trait FromPhysical<T> {
    fn from_physical(physical: T, dpi: f64) -> Self;
}

pub trait IntoLogical<T> {
    fn into_logical(self, dpi: f64) -> T;
}

impl<T, U> IntoLogical<T> for U
where
    T: FromPhysical<U>,
{
    fn into_logical(self, dpi: f64) -> T {
        T::from_physical(self, dpi)
    }
}

impl<T> IntoPhysical<(PhysicalUnit, PhysicalUnit)> for (T, T)
where
    T: Into<LogicalUnit>,
{
    fn into_physical(self, dpi: f64) -> (PhysicalUnit, PhysicalUnit) {
        let (a, b) = self;
        (a.into().into_physical(dpi), b.into().into_physical(dpi))
    }
}

impl<T> IntoLogical<(LogicalUnit, LogicalUnit)> for (T, T)
where
    T: Into<PhysicalUnit>,
{
    fn into_logical(self, dpi: f64) -> (LogicalUnit, LogicalUnit) {
        let (a, b) = self;
        (a.into().into_logical(dpi), b.into().into_logical(dpi))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct LogicalUnit(f64);

impl AsRef<f64> for LogicalUnit {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl Deref for LogicalUnit {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<f64> for LogicalUnit {
    fn from(value: f64) -> Self {
        LogicalUnit(value)
    }
}

impl From<i32> for LogicalUnit {
    fn from(value: i32) -> Self {
        LogicalUnit(value as f64)
    }
}

impl From<u32> for LogicalUnit {
    fn from(value: u32) -> Self {
        LogicalUnit(value as f64)
    }
}

impl FromPhysical<PhysicalUnit> for LogicalUnit {
    fn from_physical(physical: PhysicalUnit, dpi: f64) -> Self {
        LogicalUnit(physical.0 / dpi)
    }
}

impl Into<f64> for LogicalUnit {
    fn into(self) -> f64 {
        self.0
    }
}

impl Into<i32> for LogicalUnit {
    fn into(self) -> i32 {
        self.0.round() as i32
    }
}

impl Into<u32> for LogicalUnit {
    fn into(self) -> u32 {
        self.0.round() as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PhysicalUnit(f64);

impl AsRef<f64> for PhysicalUnit {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl Deref for PhysicalUnit {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<f64> for PhysicalUnit {
    fn from(value: f64) -> Self {
        PhysicalUnit(value)
    }
}

impl From<i32> for PhysicalUnit {
    fn from(value: i32) -> Self {
        PhysicalUnit(value as f64)
    }
}

impl From<u32> for PhysicalUnit {
    fn from(value: u32) -> Self {
        PhysicalUnit(value as f64)
    }
}

impl FromLogical<LogicalUnit> for PhysicalUnit {
    fn from_logical(logical: LogicalUnit, dpi: f64) -> Self {
        PhysicalUnit(logical.0 * dpi)
    }
}

impl Into<f64> for PhysicalUnit {
    fn into(self) -> f64 {
        self.0
    }
}

impl Into<i32> for PhysicalUnit {
    fn into(self) -> i32 {
        self.0.round() as i32
    }
}

impl Into<u32> for PhysicalUnit {
    fn into(self) -> u32 {
        self.0.round() as u32
    }
}
