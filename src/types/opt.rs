use std::fmt;

/// A convenience type to explicitly mark
/// a function argument as `opt T`.
///
/// When left unspecified on Redscript side,
/// it translates to its `Default` representation.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Opt<T> {
    /// Value is specified and guaranteed to be non-`Default` value.
    NonDefault(T),
    /// `Default` value.
    #[default]
    Default,
}

impl<T> Opt<T> {
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::NonDefault(x) => Some(x),
            Self::Default => None,
        }
    }
}

impl<T> Opt<T>
where
    T: Default,
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
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonDefault(x) => write!(f, "{}", x),
            Self::Default => write!(f, "{}", <Self as Default>::default()),
        }
    }
}

impl<T> PartialEq<T> for Opt<T>
where
    T: Default + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            Self::NonDefault(x) if x == other => true,
            Self::Default if other == &T::default() => true,
            _ => false,
        }
    }
}
