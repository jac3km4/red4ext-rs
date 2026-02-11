mod allocator;
pub mod array;
mod buffer;
mod bytecode;
mod cname;
mod cruid;
mod curve;
mod game_engine;
mod hash;
mod id;
mod node_ref;
mod opt;
mod refs;
mod res;
mod rtti;
mod stack;
mod string;
mod sync;
mod time;
mod variant;

pub use allocator::{IAllocator, PoolRef, Poolable, PoolableOps};
pub use array::{RedArray, StaticArray};
pub use buffer::{DataBuffer, DeferredDataBuffer, SharedDataBuffer};
pub use bytecode::{
    CALL_INSTR_SIZE, Instr, InvokeStatic, InvokeVirtual, OPCODE_SIZE, OpcodeHandler,
};
pub use cname::{CName, CNamePool};
pub use cruid::Cruid;
pub use curve::{Curve, MultiChannelCurve};
pub use game_engine::{GameEngine, GameInstance, NativeGameInstance, ScriptableSystem};
pub use hash::{Hash, RedHashMap};
pub use id::{
    EditorObjectId, EntityId, GameEItemIdFlag, GamedataItemStructure, Guid, ItemId, TweakDbId,
};
pub use node_ref::*;
pub use opt::Opt;
pub use refs::{Ref, ScriptRef, WeakRef};
pub use res::{MessageResourcePath, RaRef, ResRef, ResourceRef};
pub use rtti::{
    ArrayType, Bitfield, Class, ClassFlags, ClassHandle, CurveType, Enum, Function, FunctionFlags,
    FunctionHandler, GlobalFunction, IScriptable, ISerializable, Method, NativeArrayType,
    NativeClass, PointerType, Property, PropertyFlags, RaRefType, RefType, ResourceRefType,
    ScriptRefType, StaticArrayType, StaticMethod, TaggedType, Type, TypeKind, ValueContainer,
    ValuePtr, WeakRefType,
};
pub use stack::{StackArg, StackFrame};
pub use string::RedString;
pub use sync::{RwSpinLockReadGuard, RwSpinLockWriteGuard};
pub use time::{DateTime, EngineTime, GameTime};
pub use variant::Variant;

pub trait PtrEq<Rhs = Self>
where
    Rhs: ?Sized,
{
    fn ptr_eq(&self, other: &Rhs) -> bool;
}

impl<T> PtrEq for crate::red::SharedPtrBase<T> {
    #[inline]
    fn ptr_eq(&self, other: &crate::red::SharedPtrBase<T>) -> bool {
        std::ptr::eq(self.instance, other.instance)
    }
}
