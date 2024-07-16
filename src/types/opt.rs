use std::fmt;

use crate::NativeRepr;

/// A convenience type to explicitly mark
/// a function argument as `opt T`.
///
/// When left unspecified on Redscript side,
/// it translates to its `Default` representation.
#[derive(Default, Debug, Clone, PartialEq)]
pub enum Opt<T: NativeRepr> {
    /// Value is specified and guaranteed to be non-`Default` value.
    NonDefault(T),
    /// `Default` value.
    #[default]
    Default,
}

impl<T> Opt<T>
where
    T: NativeRepr,
{
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::NonDefault(x) => Some(x),
            Self::Default => None,
        }
    }
}

impl<T> Opt<T>
where
    T: NativeRepr + Default,
{
    pub fn unwrap_or_default(self) -> T {
        match self {
            Self::NonDefault(x) => x,
            Self::Default => T::default(),
        }
    }
}

impl<T> fmt::Display for Opt<T>
where
    T: fmt::Display + NativeRepr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonDefault(x) => write!(f, "{}", x),
            Self::Default => write!(f, "{}", <Self as Default>::default()),
        }
    }
}

impl<T> Copy for Opt<T> where T: NativeRepr + Copy + Clone {}

impl<T> PartialEq<T> for Opt<T>
where
    T: NativeRepr + Default + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            Self::NonDefault(x) if x == other => true,
            Self::Default if *other == T::default() => true,
            _ => false,
        }
    }
}

impl<T> PartialOrd for Opt<T>
where
    T: PartialOrd + NativeRepr + Default + PartialEq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::NonDefault(x), Opt::NonDefault(y)) => x.partial_cmp(y),
            (Self::NonDefault(x), Opt::Default) => x.partial_cmp(&T::default()),
            (Self::Default, Self::NonDefault(y)) => T::default().partial_cmp(y),
            (Self::Default, Self::Default) => Some(std::cmp::Ordering::Equal),
        }
    }
}
