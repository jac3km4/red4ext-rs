mod allocator;
pub mod array;
mod bytecode;
mod callback;
mod cname;
mod cruid;
mod engine_time;
mod entity_id;
mod game_engine;
mod game_time;
mod hash;
mod item_id;
mod misc;
mod node_ref;
mod opt;
mod refs;
mod res;
mod rtti;
mod stack;
mod string;
mod sync;
mod tweak_db_id;

pub use allocator::{IAllocator, PoolRef, Poolable, PoolableOps};
pub use array::RedArray;
pub use bytecode::{
    CALL_INSTR_SIZE, Instr, InvokeStatic, InvokeVirtual, OPCODE_SIZE, OpcodeHandler,
};
pub use callback::{Callback, VoidFunctionPointerCallback};
pub use cname::{CName, CNamePool};
pub use cruid::Cruid;
pub use engine_time::EngineTime;
pub use entity_id::EntityId;
pub use game_engine::{
    GameEngine, GameInstance, IGameSystem, IGameSystemVft, IUpdatableSystem, IUpdatableSystemVft,
    NativeGameInstance, ScriptableSystem,
};
pub use game_time::GameTime;
pub use hash::{Hash, RedHashMap};
pub use item_id::{GameEItemIdFlag, GamedataItemStructure, ItemId};
pub use misc::{
    Curve, DataBuffer, DateTime, DeferredDataBuffer, EditorObjectId, Guid, LocalizationString,
    MessageResourcePath, MultiChannelCurve, ResourceRef, SharedDataBuffer, StaticArray, Variant,
};
pub use node_ref::*;
pub use opt::Opt;
pub use refs::{Ref, ScriptRef, WeakRef};
pub use res::{RaRef, ResRef};
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
pub use tweak_db_id::TweakDbId;

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
