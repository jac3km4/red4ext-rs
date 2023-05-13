pub use red4ext_sys::error::ResourcePathError;

pub use crate::conv::{FromRepr, IntoRepr, NativeRepr, RefRepr, Strong, Weak};
pub use crate::plugin::{Plugin, Version};
pub use crate::types::{
    CName, EntityId, GameEItemIdFlag, GamedataItemStructure, IScriptable, ItemId, RedArray,
    RedString, Ref, ResRef, ScriptRef, TweakDbId, VariantExt, WRef,
};
pub use crate::{
    call, debug, define_plugin, define_trait_plugin, error, info, register_function, trace, warn,
};
#[cfg(feature = "macros")]
pub use crate::{
    macros::{redscript_global, redscript_import},
    res_ref,
};
