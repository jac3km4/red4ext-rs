pub use red4ext_sys::interop::CNameExt;
pub use red4ext_types::res_ref;

pub use crate::conv::{ClassType, FromRepr, IntoRepr, NativeRepr};
#[cfg(feature = "macros")]
pub use crate::macros::{redscript_global, redscript_import};
pub use crate::plugin::{Plugin, Version};
pub use crate::types::{
    CName, EntityId, GameEItemIdFlag, GamedataItemStructure, IScriptable, ItemId, RedArray,
    RedString, Ref, ResRef, ResourcePathError, ScriptRef, TweakDbId, Variant, VariantExt, WRef,
};
pub use crate::{
    call, debug, define_plugin, define_trait_plugin, error, info, register_function, trace, warn,
};
