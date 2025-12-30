mod opt;
pub use opt::Opt;
mod cruid;
pub use cruid::Cruid;
mod engine_time;
pub use engine_time::EngineTime;
mod entity_id;
pub use entity_id::EntityId;
mod game_time;
pub use game_time::GameTime;
mod item_id;
pub use item_id::{GameEItemIdFlag, GamedataItemStructure, ItemId};
mod node_ref;
pub use node_ref::*;
mod res;
pub use res::{RaRef, ResRef};
mod tweak_db_id;
pub use tweak_db_id::TweakDbId;
pub mod array;
pub use array::RedArray;
mod refs;
pub use refs::{Ref, ScriptRef, WeakRef};
mod string;
pub use string::RedString;
mod cname;
pub use cname::{CName, CNamePool};
mod rtti;
pub use rtti::{
    ArrayType, Bitfield, Class, ClassFlags, ClassHandle, CurveType, Enum, Function, FunctionFlags,
    FunctionHandler, GlobalFunction, IScriptable, ISerializable, Method, NativeArrayType,
    NativeClass, PointerType, Property, PropertyFlags, RaRefType, RefType, ResourceRefType,
    ScriptRefType, StaticArrayType, StaticMethod, TaggedType, Type, TypeKind, ValueContainer,
    ValuePtr, WeakRefType,
};
mod bytecode;
pub use bytecode::{
    CALL_INSTR_SIZE, Instr, InvokeStatic, InvokeVirtual, OPCODE_SIZE, OpcodeHandler,
};
mod stack;
pub use stack::{StackArg, StackFrame};
mod allocator;
pub use allocator::{IAllocator, PoolRef, Poolable, PoolableOps};
mod hash;
pub use hash::{Hash, RedHashMap};
mod sync;
pub use sync::{RwSpinLockReadGuard, RwSpinLockWriteGuard};
mod game_engine;
pub use game_engine::{GameEngine, GameInstance, NativeGameInstance, ScriptableSystem};
mod misc;
pub use misc::{
    Curve, DataBuffer, DateTime, DeferredDataBuffer, EditorObjectId, Guid, LocalizationString,
    MessageResourcePath, MultiChannelCurve, ResourceRef, SharedDataBuffer, StaticArray, Variant,
};

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
