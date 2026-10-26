use crate::raw::root::RED4ext as red;

/// SDK [CGUID](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp) layout wrapper.
#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct Guid(red::CGUID);
