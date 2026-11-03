use crate::raw::root::RED4ext as red;

/// Opaque engine date-time value.
/// Distinct from [`GameTime`] and [`EngineTime`].
///
/// SDK: [CDateTime](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct DateTime(red::CDateTime);

mod engine;
mod game;

pub use engine::*;
pub use game::*;
