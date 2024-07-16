use std::fmt;

use crate::NativeRepr;

/// A convenience type to explicitly mark
/// a function argument as `opt T`.
///
/// When left unspecified on Redscript side,
/// it translates to its `Default` representation.
#[derive(Default, Debug, Clone, PartialEq)]
pub enum OptArg<T: NativeRepr> {
    /// Value is specified and guaranteed to be non-`Default` value.
    NonDefault(T),
    /// `Default` value.
    #[default]
    Default,
}

impl<T> fmt::Display for OptArg<T>
where
    T: fmt::Display + NativeRepr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptArg::NonDefault(x) => write!(f, "{}", x),
            OptArg::Default => write!(f, "{}", <Self as Default>::default()),
        }
    }
}

impl<T> Copy for OptArg<T> where T: NativeRepr + Copy + Clone {}

impl<T> PartialEq<T> for OptArg<T>
where
    T: NativeRepr + Default + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            OptArg::NonDefault(x) if x == other => true,
            OptArg::Default if *other == T::default() => true,
            _ => false,
        }
    }
}

impl<T> PartialOrd for OptArg<T>
where
    T: PartialOrd + NativeRepr + Default + PartialEq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (OptArg::NonDefault(x), OptArg::NonDefault(y)) => x.partial_cmp(y),
            (OptArg::NonDefault(x), OptArg::Default) => x.partial_cmp(&T::default()),
            (OptArg::Default, OptArg::NonDefault(y)) => T::default().partial_cmp(y),
            (OptArg::Default, OptArg::Default) => Some(std::cmp::Ordering::Equal),
        }
    }
}
