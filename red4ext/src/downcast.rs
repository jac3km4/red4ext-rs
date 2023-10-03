//! Trait to allow for easily converting between child classes and their parent class.
//!
//! # Example
//!
//! Let's say you want to be able to call callback via `DelaySystem` with `red4ext-rs`.
//!
//! It implies that you need definitions for:
//! - [DelaySystem](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/Generated/game/DelaySystem.hpp)
//!   - let's use its method [DelayCallbackNextFrame](https://codeberg.org/adamsmasher/cyberpunk/src/branch/master/orphans.swift#L11540)
//! - [DelayCallback](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/Generated/game/DelaySystemScriptedDelayCallbackWrapper.hpp)
//!
//! First, define API in Redscript:
//!
//! ```swift,ignore
//! module My
//!
//! class CustomCallback extends Callback {
//!     private let message: String;
//!     public func Call() -> Void {
//!         LogChannel(n"DEBUG", s"received callback: \(this.message)");
//!     }
//! }
//!
//! public class System extends ScriptableSystem {
//!     // dummy constructor for callback
//!     func CreateCustomCallback(message: String) -> ref<CustomCallback> {
//!         let callback: ref<CustomCallback> = new CustomCallback();
//!         callback.message = message;
//!         return callback;
//!     }
//! }
//! ```
//!
//! Then define API wrappers with `red4ext-rs`:
//!
//! ```edition2021,ignore
//! # use red4ext_rs::prelude::{Ref, RefRepr, RedString, IScriptable, Strong};
//! # use red4ext_rs::downcast::{Downcast, TypedRef, FromTypedRef, IntoTypedRef};
//! # use red4ext_rs::macros::redscript_import;
//! // bind vanilla game classes
//!
//! #[derive(Default, Clone)]
//! #[repr(transparent)]
//! pub struct DelayCallback(Ref<IScriptable>);
//!
//! unsafe impl RefRepr for DelayCallback {
//!     const CLASS_NAME: &'static str = "gameDelaySystemScriptedDelayCallbackWrapper";
//!     type Type = Strong;
//! }
//!
//! /// ✅ do not forget to implement `FromTypedRef<Self>`
//! impl FromTypedRef<DelayCallback> for DelayCallback {
//!     fn from_typed_ref(reference: TypedRef<Self>) -> Self {
//!         Self(reference.into_inner())
//!     }
//! }
//!
//! #[derive(Default, Clone)]
//! #[repr(transparent)]
//! pub struct DelaySystem(Ref<IScriptable>);
//!
//! unsafe impl RefRepr for DelaySystem {
//!     const CLASS_NAME: &'static str = "gameDelaySystem";
//!     type Type = Strong;
//! }
//!
//! #[redscript_import]
//! impl DelaySystem {
//!     /// ⬇️ please note how, here, method only accept parent class instance
//!     #[redscript(native)]
//!     pub fn delay_callback_next_frame(&self, callback: DelayCallback) -> ();
//! }
//!
//! // bind custom classes
//!
//! #[derive(Default, Clone)]
//! #[repr(transparent)]
//! pub struct System(Ref<IScriptable>);
//!
//! unsafe impl RefRepr for System {
//!     type Type = Strong;
//!     const CLASS_NAME: &'static str = "My.System";
//! }
//!
//! #[redscript_import]
//! impl System {
//!     /// import custom callback constructor
//!     fn create_custom_callback(&self, message: RedString) -> CustomCallback;
//! }
//!
//! #[derive(Default, Clone)]
//! #[repr(transparent)]
//! pub struct CustomCallback(Ref<IScriptable>);
//!
//! unsafe impl RefRepr for CustomCallback {
//!     type Type = Strong;
//!     const CLASS_NAME: &'static str = "My.CustomCallback";
//! }
//!
//! /// downcast trait and friends make it more convenient,
//! /// ⚠️ but you MUST make sure `CustomCallback` is indeed a child class of `DelayCallback`.
//! unsafe impl IntoTypedRef<DelayCallback> for CustomCallback {
//!     fn into_typed_ref(self) -> TypedRef<DelayCallback> {
//!         TypedRef::new(self.0)
//!     }
//! }
//!
//! impl System {
//!     /// then, whenever you need to trigger your callback
//!     fn when_something_happens(&self) {
//!         let callback: CustomCallback = self.create_custom_callback(RedString::new("Hello from My.System!"));
//!         // ✅ now simply `downcast` your child class
//!         self.delay_callback_next_frame(callback.downcast());
//!     }
//! }
//! ```

use std::marker::PhantomData;

use super::{conv::RefRepr, ffi::IScriptable, types::Ref};

#[derive(Default, Clone)]
#[repr(transparent)]
pub struct TypedRef<T>(Ref<IScriptable>, PhantomData<T>);

impl<T> TypedRef<T> {
    pub fn new(reference: Ref<IScriptable>) -> Self {
        Self(reference, PhantomData)
    }
    pub fn into_inner(self) -> Ref<IScriptable> {
        self.0
    }
}

/// SAFETY: implementations of this trait are only valid if your implementors are indeed child classes of `Parent`
pub unsafe trait IntoTypedRef<Parent: RefRepr + Default + Clone> {
    /// cast a reference into a parent typed reference
    fn into_typed_ref(self) -> TypedRef<Parent>;
}

pub trait FromTypedRef<Parent: RefRepr + Default + Clone>: Sized {
    /// cast any self typed reference into itself
    fn from_typed_ref(reference: TypedRef<Parent>) -> Self;
}

pub trait Downcast<Parent>: IntoTypedRef<Parent>
where
    Self: IntoTypedRef<Parent>,
    Parent: RefRepr + Default + Clone + FromTypedRef<Parent>,
{
    /// automatically downcast a wrapper over `Ref<IScriptable>`
    /// into its parent
    fn downcast(self) -> Parent
    where
        Self: Sized,
    {
        Parent::from_typed_ref(self.into_typed_ref())
    }
}

/// automatically implements downcast for any child classes that can be turned into their parent class
impl<Child, Parent> Downcast<Parent> for Child
where
    Self: IntoTypedRef<Parent>,
    Parent: RefRepr + Default + Clone + FromTypedRef<Parent>,
{
}
