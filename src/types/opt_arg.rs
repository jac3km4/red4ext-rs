use std::fmt;

use crate::NativeRepr;

/// A convenience type to explicitly mark
/// a function argument as `opt T`.
///
/// > When left unspecified on Redscript side,
/// it translates to its `Default` representation.
#[derive(Default)]
pub enum OptArg<T: NativeRepr + Default + PartialEq> {
    /// type is specified and guarantee to be non-`Default` value.
    NonDefault(T),
    /// type is guaranteed to be `Default` value.
    #[default]
    Default,
}

impl<T> fmt::Debug for OptArg<T>
where
    T: fmt::Debug + NativeRepr + Default + PartialEq,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonDefault(arg0) => f.debug_tuple("NonDefault").field(arg0).finish(),
            Self::Default => write!(f, "Default"),
        }
    }
}

impl<T> fmt::Display for OptArg<T>
where
    T: fmt::Display + NativeRepr + Default + PartialEq,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptArg::NonDefault(x) => write!(f, "{}", x),
            OptArg::Default => write!(f, "{}", <Self as Default>::default()),
        }
    }
}

impl<T> Clone for OptArg<T>
where
    T: Clone + NativeRepr + Default + PartialEq,
{
    fn clone(&self) -> Self {
        match self {
            Self::NonDefault(arg0) => Self::NonDefault(arg0.clone()),
            Self::Default => Self::Default,
        }
    }
}

impl<T> Copy for OptArg<T> where T: Copy + Clone + NativeRepr + Default + PartialEq {}
