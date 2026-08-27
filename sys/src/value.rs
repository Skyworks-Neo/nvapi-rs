use crate::ArgumentRangeError;
use std::hash::{Hash, Hasher};
use std::mem::transmute;
use std::ops;
use std::{fmt, iter};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub use nvapi_macros::{NvValueBits, NvValueData, NvValueEnum};

/// Alias of [`NvValue<T>`] as emitted by `nvenum!` (`type NV_X = NvEnum<T>`).
/// Same transparent newtype — the name documents that the alias field holds a
/// single enum discriminant, nothing more.
pub type NvEnum<T> = NvValue<T>;
/// Alias of [`NvValue<T>`] as emitted by `nvbits!` (`type NV_X = NvBits<T>`).
/// Same transparent newtype — the name documents that the alias field holds a
/// bitflags value, nothing more.
pub type NvBits<T> = NvValue<T>;

pub trait NvValueData:
    Copy
    + PartialEq
    + Eq
    + Into<<Self as NvValueData>::Repr>
    + TryFrom<<Self as NvValueData>::Repr, Error = ArgumentRangeError>
{
    const NAME: &'static str;
    const C_NAME: &'static str;
    fn all_values() -> &'static [Self];

    type Repr: Copy + PartialEq + Eq + fmt::Display;

    fn values() -> iter::Copied<std::slice::Iter<'static, Self>> {
        Self::all_values().iter().copied()
    }

    fn repr(self) -> Self::Repr;
    fn repr_ref(&self) -> &Self::Repr;
    fn from_repr(value: Self::Repr) -> Result<Self, ArgumentRangeError>;
    fn from_repr_ref(value: &Self::Repr) -> Result<&Self, ArgumentRangeError>;
    fn from_repr_mut(value: &mut Self::Repr) -> Result<&mut Self, ArgumentRangeError>;

    fn value(self) -> NvValue<Self> {
        NvValue::new(self)
    }
}

pub trait NvValueEnum: NvValueData {}

pub trait NvValueBits: NvValueData {
    fn from_repr_truncate(value: Self::Repr) -> Self;
}

#[derive(FromBytes, PartialEq, Eq, Immutable)]
#[repr(transparent)]
pub struct NvValue<T: NvValueData> {
    pub value: T::Repr,
}

impl<T: NvValueData> NvValue<T> {
    pub fn new(value: T) -> Self {
        Self::with_repr(value.repr())
    }

    pub const fn with_repr(value: T::Repr) -> Self {
        Self { value }
    }

    pub const fn with_repr_ref(value: &T::Repr) -> &Self {
        unsafe { transmute(value) }
    }

    pub fn with_repr_mut(value: &mut T::Repr) -> &mut Self {
        unsafe { transmute(value) }
    }

    pub fn cast<U: NvValueData<Repr = T::Repr>>(self) -> NvValue<U> {
        NvValue::with_repr(self.repr())
    }

    pub fn try_get(self) -> Result<T, ArgumentRangeError> {
        T::try_from(self.value)
    }

    pub fn try_ref(&self) -> Result<&T, ArgumentRangeError> {
        T::from_repr_ref(self.repr_ref())
    }

    pub fn try_mut(&mut self) -> Result<&mut T, ArgumentRangeError> {
        T::from_repr_mut(self.repr_mut())
    }

    pub const fn repr(self) -> T::Repr {
        self.value
    }

    pub const fn repr_ref(&self) -> &T::Repr {
        &self.value
    }

    pub fn repr_mut(&mut self) -> &mut T::Repr {
        &mut self.value
    }

    pub fn display(&self) -> &dyn fmt::Display
    where
        T: fmt::Display,
    {
        match self.try_ref() {
            Ok(value) => value,
            Err(..) => self.repr_ref(),
        }
    }
}

impl<T: NvValueBits> NvValue<T> {
    pub fn truncate(&self) -> T {
        T::from_repr_truncate(self.repr())
    }
}

impl<T: NvValueData> From<T> for NvValue<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<'a, T: NvValueData> From<&'a T> for &'a NvValue<T> {
    fn from(value: &'a T) -> Self {
        NvValue::with_repr_ref(value.repr_ref())
    }
}

impl<'a, T: NvValueData> From<&'a NvValue<T>> for NvValue<T> {
    fn from(value: &'a NvValue<T>) -> Self {
        *value
    }
}

unsafe impl<T: NvValueData> IntoBytes for NvValue<T>
where
    T::Repr: IntoBytes,
{
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized,
    {
    }
}

impl<T: NvValueData> Copy for NvValue<T> {}
impl<T: NvValueData> Clone for NvValue<T>
where
    T::Repr: Clone,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: NvValueData> Default for NvValue<T>
where
    T::Repr: Default,
{
    fn default() -> Self {
        Self {
            value: Default::default(),
        }
    }
}

impl<T: NvValueData> PartialOrd for NvValue<T>
where
    T::Repr: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: NvValueData> Ord for NvValue<T>
where
    T::Repr: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: NvValueData> PartialEq<T> for NvValue<T> {
    fn eq(&self, other: &T) -> bool {
        self.value.eq(other.repr_ref())
    }
}

impl<T: NvValueData> Hash for NvValue<T>
where
    T::Repr: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<T: NvValueData> fmt::Display for NvValue<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match T::try_from(self.value) {
            Ok(value) => fmt::Display::fmt(&value, f),
            Err(..) => fmt::Display::fmt(&self.value, f),
        }
    }
}

impl<T: NvValueData> fmt::Debug for NvValue<T>
where
    T: fmt::Debug,
    T::Repr: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug;
        match T::try_from(self.value) {
            Ok(value) => {
                debug = f.debug_tuple(T::NAME);
                debug.field(&value);
            }
            Err(..) => {
                debug = f.debug_tuple(T::C_NAME);
            }
        }
        debug.field(&self.value).finish()
    }
}

impl<T: NvValueData> fmt::LowerHex for NvValue<T>
where
    T::Repr: fmt::LowerHex,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.repr(), f)
    }
}

impl<T: NvValueData> fmt::UpperHex for NvValue<T>
where
    T::Repr: fmt::UpperHex,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.repr(), f)
    }
}

impl<T: NvValueData> ops::Not for NvValue<T>
where
    T::Repr: ops::Not,
    <T::Repr as ops::Not>::Output: Into<T::Repr>,
{
    type Output = Self;
    fn not(self) -> Self::Output {
        let value = !self.repr();
        Self::with_repr(value.into())
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitOr<Rhs> for NvValue<T>
where
    T::Repr: ops::BitOr,
    <T::Repr as ops::BitOr>::Output: Into<T::Repr>,
{
    type Output = Self;
    fn bitor(self, rhs: Rhs) -> Self::Output {
        let value = self.repr() | rhs.into();
        Self::with_repr(value.into())
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitAnd<Rhs> for NvValue<T>
where
    T::Repr: ops::BitAnd,
    <T::Repr as ops::BitAnd>::Output: Into<T::Repr>,
{
    type Output = Self;
    fn bitand(self, rhs: Rhs) -> Self::Output {
        let value = self.repr() & rhs.into();
        Self::with_repr(value.into())
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitXor<Rhs> for NvValue<T>
where
    T::Repr: ops::BitXor,
    <T::Repr as ops::BitXor>::Output: Into<T::Repr>,
{
    type Output = Self;
    fn bitxor(self, rhs: Rhs) -> Self::Output {
        let value = self.repr() ^ rhs.into();
        Self::with_repr(value.into())
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitOrAssign<Rhs> for NvValue<T>
where
    T::Repr: ops::BitOrAssign,
{
    fn bitor_assign(&mut self, rhs: Rhs) {
        self.value |= rhs.into();
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitAndAssign<Rhs> for NvValue<T>
where
    T::Repr: ops::BitAndAssign,
{
    fn bitand_assign(&mut self, rhs: Rhs) {
        self.value &= rhs.into();
    }
}

impl<T: NvValueData, Rhs: Into<T::Repr>> ops::BitXorAssign<Rhs> for NvValue<T>
where
    T::Repr: ops::BitXorAssign,
{
    fn bitxor_assign(&mut self, rhs: Rhs) {
        self.value ^= rhs.into();
    }
}

#[cfg(feature = "serde")]
mod serde_impl_nvenum {
    use super::{NvValue, NvValueData};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<'de, T: NvValueData> Deserialize<'de> for NvValue<T>
    where
        T::Repr: Deserialize<'de>,
    {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            Deserialize::deserialize(de).map(Self::with_repr)
        }
    }

    impl<T: NvValueData> Serialize for NvValue<T>
    where
        T::Repr: Serialize,
    {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            self.value.serialize(ser)
        }
    }
}

macro_rules! nvvalue_reprs {
    ($($repr:ty,)*) => { $(
        impl<T: NvValueData<Repr=$repr>> From<NvValue<T>> for $repr {
            fn from(value: NvValue<T>) -> Self {
                value.repr()
            }
        }

        impl<T: NvValueData<Repr=$repr>> From<$repr> for NvValue<T> {
            fn from(value: T::Repr) -> Self {
                Self::with_repr(value)
            }
        }

        impl<T: NvValueData<Repr=$repr>> PartialEq<$repr> for NvValue<T> {
            fn eq(&self, other: &T::Repr) -> bool {
                self.value.eq(other)
            }
        }
    )* };
}

nvvalue_reprs! {
    u32, i32,
}

/// Pin the serde wire format of the two enum flavors introduced by the
/// `nvenum!`/`nvbits!` proc-macros, so downstream JSON consumers (nvoc GUI/TUI
/// dashboards) don't silently break:
///
/// * the typed enum (`SystemType`) derives `Serialize`/`Deserialize` and
///   serializes as a variant-name JSON string — same as the pre-migration
///   `macro_rules!` output;
/// * the FFI alias (`NV_SYSTEM_TYPE`, an `NvEnum`/`NvValue` newtype) is
///   transparently serialized as the raw repr integer — matching the old bare
///   `c_int` alias fields;
/// * `nvbits!` flags serialize as bitflags' `{"bits": N}` object.
///
/// Note the asymmetry this implies: alias integers deserialize *unknown* values
/// fine (transparent passthrough), while typed enum strings reject unknown
/// variant names.
#[cfg(all(test, feature = "serde"))]
mod serde_format_tests {
    use crate::gpu::display::ConnectedIdsFlags;
    use crate::gpu::{NV_SYSTEM_TYPE_DESKTOP, NV_SYSTEM_TYPE_LAPTOP, SystemType};
    use serde_json::json;

    #[test]
    fn typed_enum_serializes_as_variant_name_string() {
        assert_eq!(
            serde_json::to_value(SystemType::Laptop).unwrap(),
            json!("Laptop")
        );
        assert_eq!(
            serde_json::to_value(SystemType::Desktop).unwrap(),
            json!("Desktop")
        );

        let round: SystemType =
            serde_json::from_str("\"Laptop\"").expect("variant-name string must round-trip");
        assert_eq!(round, SystemType::Laptop);

        serde_json::from_str::<SystemType>("\"NotAVariant\"")
            .expect_err("unknown variant names must be rejected");
    }

    #[test]
    fn enum_alias_serializes_as_raw_repr_integer() {
        assert_eq!(
            serde_json::to_value(NV_SYSTEM_TYPE_LAPTOP).unwrap(),
            json!(1)
        );
        assert_eq!(
            serde_json::to_value(NV_SYSTEM_TYPE_DESKTOP).unwrap(),
            json!(2)
        );

        let round: crate::gpu::NV_SYSTEM_TYPE =
            serde_json::from_str("1").expect("repr integer must round-trip");
        assert_eq!(round, NV_SYSTEM_TYPE_LAPTOP);

        let unknown: crate::gpu::NV_SYSTEM_TYPE =
            serde_json::from_str("99").expect("unknown reprs pass through transparently");
        assert_eq!(unknown.repr(), 99);
    }

    #[test]
    fn bits_flags_serialize_as_bitflags_object() {
        let flags = ConnectedIdsFlags::UNCACHED | ConnectedIdsFlags::SLI;
        assert_eq!(serde_json::to_value(flags).unwrap(), json!({"bits": 3}));

        let round: ConnectedIdsFlags =
            serde_json::from_str("{\"bits\":3}").expect("bitflags object must round-trip");
        assert_eq!(round, flags);
    }
}

/// Core NvValue semantics (audit #17): unknown reprs must flow through
/// try_get as errors, display as the raw number, and compare/hash by repr —
/// the transparent-alias contract the CLI rendering depends on.
#[cfg(test)]
mod nvvalue_tests {
    use super::NvValue;
    use crate::gpu::{NV_SYSTEM_TYPE_LAPTOP, SystemType};
    use std::collections::HashSet;

    #[test]
    fn try_get_known_and_unknown() {
        let known = NvValue::<SystemType>::with_repr(NV_SYSTEM_TYPE_LAPTOP.value);
        assert_eq!(known.try_get(), Ok(SystemType::Laptop));
        let unknown = NvValue::<SystemType>::with_repr(99);
        assert!(unknown.try_get().is_err());
        assert!(unknown.try_ref().is_err());
    }

    #[test]
    fn display_falls_back_to_repr() {
        let unknown = NvValue::<SystemType>::with_repr(99);
        assert_eq!(format!("{}", unknown.display()), "99");
        let known = NvValue::<SystemType>::with_repr(NV_SYSTEM_TYPE_LAPTOP.value);
        assert_eq!(format!("{}", known.display()), "Laptop");
    }

    #[test]
    fn partial_eq_compares_repr() {
        assert_eq!(
            NvValue::<SystemType>::with_repr(99),
            NvValue::<SystemType>::with_repr(99)
        );
        assert_ne!(
            NvValue::<SystemType>::with_repr(99),
            NvValue::<SystemType>::with_repr(1)
        );
    }

    #[test]
    fn unknown_repr_hashes_distinctly() {
        let mut set = HashSet::new();
        set.insert(NvValue::<SystemType>::with_repr(99));
        set.insert(NvValue::<SystemType>::with_repr(1));
        set.insert(NvValue::<SystemType>::with_repr(99));
        assert_eq!(set.len(), 2);
    }
}
