//! Access to engine systems and RTTI registration.
//!
//! Engine singletons require initialized game systems; plugin load alone is not sufficient.

mod rtti;
pub use rtti::{RttiRegistrator, RttiSystem, RttiSystemMut};
