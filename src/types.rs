use crate::sys;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::ops::RangeInclusive;
use std::{fmt, ops};

fn fmt_scaled(value: f32, unit: &str, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(precision) = f.precision() {
        write!(f, "{:.*} {}", precision, value, unit)
    } else {
        write!(f, "{} {}", value, unit)
    }
}

fn fmt_debug<T: fmt::Display>(v: &T, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Display::fmt(v, f)
}

macro_rules! debug_via_display {
    ($ty:ty) => {
        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt_debug(self, f)
            }
        }
    };
}

pub trait RawConversion {
    type Target;
    type Error;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error>;
}

impl<const N: usize> RawConversion for sys::types::NvString<N> {
    type Target = String;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        Ok(self.to_string_lossy().into_owned())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Celsius(pub i32);

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}C", self.0)
    }
}
debug_via_display!(Celsius);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Rpm(pub u32);

impl fmt::Display for Rpm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} RPM", self.0)
    }
}
debug_via_display!(Rpm);

/// Nvidia encodes temperature as `<< 8` for some reason sometimes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct CelsiusShifted(pub i32);

impl fmt::Display for CelsiusShifted {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}C", self.get())
    }
}
debug_via_display!(CelsiusShifted);

impl CelsiusShifted {
    pub fn get(&self) -> i32 {
        self.0 >> 8
    }
}

impl From<CelsiusShifted> for Celsius {
    fn from(c: CelsiusShifted) -> Self {
        Celsius(c.get())
    }
}

impl From<Celsius> for CelsiusShifted {
    fn from(c: Celsius) -> Self {
        CelsiusShifted(c.0 << 8)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Microvolts(pub u32);

impl fmt::Display for Microvolts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_scaled(self.0 as f32 / 1000.0, "mV", f)
    }
}
debug_via_display!(Microvolts);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct MicrovoltsDelta(pub i32);

impl fmt::Display for MicrovoltsDelta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_scaled(self.0 as f32 / 1000.0, "mV", f)
    }
}
debug_via_display!(MicrovoltsDelta);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Kilohertz(pub u32);

impl From<u32> for Kilohertz {
    fn from(p: u32) -> Self {
        Kilohertz(p)
    }
}

impl fmt::Display for Kilohertz {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 < 1000 {
            write!(f, "{} kHz", self.0)
        } else {
            fmt_scaled(self.0 as f32 / 1000.0, "MHz", f)
        }
    }
}
debug_via_display!(Kilohertz);

impl ops::Sub for Kilohertz {
    type Output = KilohertzDelta;

    fn sub(self, rhs: Self) -> Self::Output {
        KilohertzDelta(self.0 as i32 - rhs.0 as i32)
    }
}

impl ops::Sub<KilohertzDelta> for Kilohertz {
    type Output = Kilohertz;

    fn sub(self, rhs: KilohertzDelta) -> Self::Output {
        Kilohertz((self.0 as i32 - rhs.0) as u32)
    }
}

impl ops::Add<KilohertzDelta> for Kilohertz {
    type Output = Kilohertz;

    fn add(self, rhs: KilohertzDelta) -> Self::Output {
        Kilohertz((self.0 as i32 + rhs.0) as u32)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Kilohertz2(pub u32);

impl Kilohertz2 {
    pub fn get(&self) -> u32 {
        self.0 / 2
    }
}

impl From<Kilohertz2> for Kilohertz {
    fn from(p: Kilohertz2) -> Self {
        Kilohertz(p.get())
    }
}

impl From<Kilohertz> for Kilohertz2 {
    fn from(v: Kilohertz) -> Self {
        Kilohertz2(v.0 * 2)
    }
}

impl From<u32> for Kilohertz2 {
    fn from(p: u32) -> Self {
        Kilohertz2(p)
    }
}

impl fmt::Display for Kilohertz2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = self.get();
        if v < 1000 {
            write!(f, "{} kHz", v)
        } else {
            fmt_scaled(v as f32 / 1000.0, "MHz", f)
        }
    }
}
debug_via_display!(Kilohertz2);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct KilohertzDelta(pub i32);

impl From<i32> for KilohertzDelta {
    fn from(p: i32) -> Self {
        KilohertzDelta(p)
    }
}

impl fmt::Display for KilohertzDelta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.abs() < 1000 {
            write!(f, "{} kHz", self.0)
        } else {
            fmt_scaled(self.0 as f32 / 1000.0, "MHz", f)
        }
    }
}
debug_via_display!(KilohertzDelta);

impl ops::Add for KilohertzDelta {
    type Output = KilohertzDelta;

    fn add(self, rhs: Self) -> Self::Output {
        KilohertzDelta(self.0 + rhs.0)
    }
}

impl ops::Sub for KilohertzDelta {
    type Output = KilohertzDelta;

    fn sub(self, rhs: Self) -> Self::Output {
        KilohertzDelta(self.0 - rhs.0)
    }
}

impl ops::Mul<i32> for KilohertzDelta {
    type Output = KilohertzDelta;

    fn mul(self, rhs: i32) -> Self::Output {
        KilohertzDelta(self.0 * rhs)
    }
}

impl ops::Div<i32> for KilohertzDelta {
    type Output = KilohertzDelta;

    fn div(self, rhs: i32) -> Self::Output {
        KilohertzDelta(self.0 / rhs)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Kilohertz2Delta(pub i32);

impl From<i32> for Kilohertz2Delta {
    fn from(p: i32) -> Self {
        Kilohertz2Delta(p)
    }
}

impl Kilohertz2Delta {
    pub fn get(&self) -> i32 {
        self.0 / 2
    }
}

impl From<Kilohertz2Delta> for KilohertzDelta {
    fn from(p: Kilohertz2Delta) -> Self {
        KilohertzDelta(p.get())
    }
}

impl From<KilohertzDelta> for Kilohertz2Delta {
    fn from(v: KilohertzDelta) -> Self {
        Kilohertz2Delta(v.0 * 2)
    }
}

impl fmt::Display for Kilohertz2Delta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = self.get();
        if v.abs() < 1000 {
            write!(f, "{} kHz", v)
        } else {
            fmt_scaled(v as f32 / 1000.0, "MHz", f)
        }
    }
}
debug_via_display!(Kilohertz2Delta);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Kibibytes(pub u32);

impl ops::Sub for Kibibytes {
    type Output = Kibibytes;

    fn sub(self, rhs: Self) -> Self::Output {
        Kibibytes(self.0 - rhs.0)
    }
}

impl fmt::Display for Kibibytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 < 1000 {
            write!(f, "{} KiB", self.0)
        } else if self.0 < 1000000 {
            let value = self.0 as f32 / 1024.0;
            if let Some(precision) = f.precision() {
                write!(f, "{:.*} MiB", precision, value)
            } else {
                write!(f, "{} MiB", value)
            }
        } else {
            let value = self.0 as f32 / 1048576.0;
            if let Some(precision) = f.precision() {
                write!(f, "{:.*} GiB", precision, value)
            } else {
                write!(f, "{} GiB", value)
            }
        }
    }
}
debug_via_display!(Kibibytes);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Percentage(pub u32);

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}
debug_via_display!(Percentage);

impl Percentage {
    pub fn from_raw(v: u32) -> Result<Self, sys::ArgumentRangeError> {
        match v {
            v @ 0..=100 => Ok(Percentage(v)),
            _ => Err(sys::ArgumentRangeError::new(v as _)),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Percentage1000(pub u32);

impl fmt::Display for Percentage1000 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_scaled(self.0 as f32 / 1000.0, "%", f)
    }
}
debug_via_display!(Percentage1000);

impl Percentage1000 {
    pub fn get(&self) -> u32 {
        self.0 / 1000
    }
}

impl From<Percentage1000> for Percentage {
    fn from(p: Percentage1000) -> Self {
        Percentage(p.get())
    }
}

impl From<Percentage> for Percentage1000 {
    fn from(p: Percentage) -> Self {
        Percentage1000(p.0 * 1000)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

impl<T: fmt::Display> fmt::Display for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ~ {}", self.min, self.max)
    }
}

impl<T: fmt::Debug> fmt::Debug for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} ~ {:?}", self.min, self.max)
    }
}

impl<T> Range<T> {
    pub fn range_from<U>(r: Range<U>) -> Self
    where
        T: From<U>,
    {
        Range {
            min: r.min.into(),
            max: r.max.into(),
        }
    }

    pub fn from_scalar(v: T) -> Self
    where
        T: Clone,
    {
        Range {
            min: v.clone(),
            max: v,
        }
    }

    pub fn range(&self) -> RangeInclusive<T>
    where
        T: Clone,
    {
        self.min.clone()..=self.max.clone()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Delta<T> {
    pub value: T,
    pub range: Range<T>,
}

#[cfg(test)]
mod percentage_tests {
    use super::{Percentage, Percentage1000};

    #[test]
    fn from_raw_bounds() {
        assert_eq!(Percentage::from_raw(0), Ok(Percentage(0)));
        assert_eq!(Percentage::from_raw(100), Ok(Percentage(100)));
        assert!(Percentage::from_raw(101).is_err());
        assert!(Percentage::from_raw(u32::MAX).is_err());
    }

    #[test]
    fn percentage1000_roundtrip() {
        let p = Percentage(50);
        let p1000: Percentage1000 = p.into();
        assert_eq!(p1000, Percentage1000(50_000));
        let back: Percentage = p1000.into();
        assert_eq!(back, p);
    }
}
