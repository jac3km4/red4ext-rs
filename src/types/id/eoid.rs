use crate::raw::root::RED4ext as red;

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct EditorObjectId(red::EditorObjectID);
