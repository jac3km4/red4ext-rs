use std::fmt;

/// Rust adapter for REDscript `opt T` parameters.
/// Omitted arguments use `T::default()`, so omission and an explicit default value
/// cannot be distinguished during representation conversion. Comparisons and
/// display treat [`Opt::Default`] as `T::default()`.
///
/// SDK: [parameter flags](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/CProperty.hpp).
#[derive(Default, Debug, Clone, Copy)]
pub enum Opt<T> {
    /// Explicit value. Direct construction does not enforce a non-default value.
    NonDefault(T),
    /// Uses `T::default()` when converted to an engine representation.
    #[default]
    Default,
}

impl<T> From<T> for Opt<T>
where
    T: Default + PartialEq,
{
    fn from(value: T) -> Self {
        if value == T::default() {
            return Opt::Default;
        }
        Opt::NonDefault(value)
    }
}

impl<T> Opt<T> {
    /// Returns the explicit value, or `None` for [`Opt::Default`].
    /// Unlike comparisons, this preserves an explicitly stored default value.
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
    /// Returns the explicit value, or `T::default()` for [`Opt::Default`].
    pub fn unwrap_or_default(self) -> T {
        match self {
            Self::NonDefault(x) => x,
            Self::Default => T::default(),
        }
    }
}

impl<T> fmt::Display for Opt<T>
where
    T: fmt::Display + Default,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonDefault(x) => write!(f, "{x}"),
            Self::Default => write!(f, "{}", T::default()),
        }
    }
}

impl<T> PartialEq for Opt<T>
where
    T: Default + PartialEq,
{
    fn eq(&self, other: &Opt<T>) -> bool {
        match (self, other) {
            (Opt::NonDefault(lhs), Opt::NonDefault(rhs)) => lhs.eq(rhs),
            (Opt::NonDefault(lhs), Opt::Default) => lhs.eq(&T::default()),
            (Opt::Default, Opt::NonDefault(rhs)) => T::default().eq(rhs),
            (Opt::Default, Opt::Default) => true,
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

impl<T> Eq for Opt<T> where T: Eq + Default {}

impl<T> PartialOrd for Opt<T>
where
    T: Default + PartialOrd + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialOrd<T> for Opt<T>
where
    T: Default + PartialOrd + Ord,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::NonDefault(lhs), rhs) => Some(lhs.cmp(rhs)),
            (Self::Default, rhs) => Some(T::default().cmp(rhs)),
        }
    }
}

impl<T> Ord for Opt<T>
where
    T: Default + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::NonDefault(lhs), Self::NonDefault(rhs)) => lhs.cmp(rhs),
            (Self::NonDefault(lhs), Self::Default) => lhs.cmp(&T::default()),
            (Self::Default, Self::NonDefault(rhs)) => T::default().cmp(rhs),
            (Self::Default, Self::Default) => std::cmp::Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Opt;

    #[test]
    fn display() {
        assert_eq!(Opt::<i32>::Default.to_string(), "0");
        assert_eq!(Opt::NonDefault(42).to_string(), "42");
        assert_eq!(Opt::<String>::Default.to_string(), "");
    }

    #[test]
    fn transitivity() {
        assert!(Opt::NonDefault(0).eq(&Opt::Default));
        assert!(Opt::<i32>::Default.eq(&Opt::NonDefault(0)));
        assert_eq!(
            Opt::NonDefault(0).cmp(&Opt::Default),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            Opt::<i32>::Default.cmp(&Opt::NonDefault(0)),
            std::cmp::Ordering::Equal
        );
    }
}
