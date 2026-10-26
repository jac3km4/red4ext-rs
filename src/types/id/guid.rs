use crate::raw::root::RED4ext as red;

/// Opaque GUID value.
/// [`Default`] does not generate a unique identifier.
///
/// SDK: [CGUID](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct Guid(red::CGUID);
