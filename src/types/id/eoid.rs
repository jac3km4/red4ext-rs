use crate::raw::root::RED4ext as red;

/// Opaque editor object identifier.
///
/// SDK: [EditorObjectID](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct EditorObjectId(red::EditorObjectID);
