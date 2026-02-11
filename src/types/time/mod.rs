use crate::raw::root::RED4ext as red;

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct DateTime(red::CDateTime);

mod engine;
mod game;

pub use engine::*;
pub use game::*;
