#![allow(clippy::missing_safety_doc)]
mod raw;
use std::hash::Hash;
use std::sync::OnceLock;
use std::{ffi, fmt, iter, mem, ops, ptr};

use raw::root::versioning::constants;
use raw::root::RED4ext as red;
pub use widestring::{widecstr as wcstr, U16CStr};

pub mod hashes {
    pub use super::red::Detail::AddressHashes::*;

    #[inline]
    pub fn resolve(hash: u32) -> usize {
        unsafe { super::red::UniversalRelocBase::Resolve(hash) }
    }
}

#[doc(hidden)]
pub mod internal {
    pub use crate::red::{EMainReason, PluginHandle, PluginInfo, Sdk};
}

#[derive(Debug)]
pub struct SemVer(red::SemVer);

impl SemVer {
    #[inline]
    pub const fn new(major: u8, minor: u16, patch: u32) -> Self {
        Self(red::SemVer {
            major,
            minor,
            patch,
            prerelease: red::v0::SemVer_PrereleaseInfo {
                type_: 0,
                number: 0,
            },
        })
    }
}

#[derive(Debug)]
pub struct RuntimeVersion(red::FileVer);

impl RuntimeVersion {
    const RUNTIME_INDEPENDENT: Self = Self(red::FileVer {
        major: constants::RUNTIME_INDEPENDENT as _,
        minor: constants::RUNTIME_INDEPENDENT as _,
        build: constants::RUNTIME_INDEPENDENT as _,
        revision: constants::RUNTIME_INDEPENDENT as _,
    });
}

#[derive(Debug)]
pub struct SdkVersion(SemVer);

impl SdkVersion {
    const LATEST: Self = Self(SemVer::new(
        constants::SDK_MAJOR as _,
        constants::SDK_MINOR as _,
        constants::SDK_PATCH as _,
    ));
}

#[derive(Debug)]
pub struct ApiVersion(u32);

impl ApiVersion {
    const LATEST: Self = Self(constants::API_VERSION_LATEST as _);
}

impl From<ApiVersion> for u32 {
    #[inline]
    fn from(api: ApiVersion) -> u32 {
        api.0
    }
}

macro_rules! log_internal {
    ($self:ident, $level:ident, $msg:expr) => {
        unsafe {
            let str = truncated_cstring($msg.to_string());
            ((*$self.sdk.logger).$level.unwrap())($self.handle, str.as_ptr());
        }
    };
}

#[derive(Debug)]
pub struct SdkEnv {
    handle: red::PluginHandle,
    sdk: red::Sdk,
}

impl SdkEnv {
    pub fn new(handle: red::PluginHandle, sdk: red::Sdk) -> Self {
        Self { handle, sdk }
    }

    #[inline]
    pub fn info(&self, txt: impl fmt::Display) {
        log_internal!(self, Info, txt);
    }

    #[inline]
    pub fn warn(&self, txt: impl fmt::Display) {
        log_internal!(self, Warn, txt);
    }

    #[inline]
    pub fn error(&self, txt: impl fmt::Display) {
        log_internal!(self, Error, txt);
    }

    #[inline]
    pub fn debug(&self, txt: impl fmt::Display) {
        log_internal!(self, Debug, txt);
    }

    #[inline]
    pub fn add_listener(&self, typ: StateType, mut listener: StateListener) -> bool {
        unsafe { ((*self.sdk.gameStates).Add.unwrap())(self.handle, typ as _, &mut listener.0) }
    }

    pub unsafe fn attach_hook<F1, A1, R1, F2, A2, R2>(
        &self,
        hook: *mut Hook<F1, F2>,
        target: F1,
        detour: F2,
    ) -> bool
    where
        F1: FnPtr<A1, R1>,
        F2: FnPtr<A2, R2>,
    {
        unsafe {
            let Hook(original, cb_ref, detour_ref) = &*hook;
            detour_ref.replace(Some(detour));

            ((*self.sdk.hooking).Attach.unwrap())(
                self.handle,
                target.to_ptr(),
                original.to_ptr(),
                (*cb_ref) as _,
            )
        }
    }

    #[inline]
    pub unsafe fn detach_hook<F, A, R>(&self, target: F) -> bool
    where
        F: FnPtr<A, R>,
    {
        unsafe { ((*self.sdk.hooking).Detach.unwrap())(self.handle, target.to_ptr()) }
    }
}

unsafe impl Send for SdkEnv {}
unsafe impl Sync for SdkEnv {}

pub mod log {
    pub use crate::{debug, error, info, warn};

    #[macro_export]
    macro_rules! info {
        ($env:expr, $($arg:tt)*) => {
            $env.info(format_args!($($arg)*))
        };
    }

    #[macro_export]
    macro_rules! warn {
        ($env:expr, $($arg:tt)*) => {
            $env.warn(format_args!($($arg)*))
        };
    }

    #[macro_export]
    macro_rules! error {
        ($env:expr, $($arg:tt)*) => {
            $env.error(format_args!($($arg)*))
        };
    }

    #[macro_export]
    macro_rules! debug {
        ($env:expr, $($arg:tt)*) => {
            $env.debug(format_args!($($arg)*))
        };
    }
}

#[macro_export]
macro_rules! hooks {
    ($(static $name:ident: fn($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {$(
        static mut $name: *mut $crate::Hook<
            unsafe extern "C" fn($($arg: $ty),*) -> $ret,
            unsafe extern "C" fn($($arg: $ty),*, cb: unsafe extern "C" fn($($arg: $ty),*) -> $ret) -> $ret
        > = unsafe {

            static mut TARGET: Option<unsafe extern "C" fn($($arg: $ty),*) -> $ret> = None;
            static mut DETOUR: Option<unsafe extern "C" fn($($arg: $ty),*, cb: unsafe extern "C" fn($($arg: $ty),*) -> $ret) -> $ret> = None;

            unsafe extern "C" fn internal($($arg: $ty),*) -> $ret {
                let target = unsafe { TARGET.expect("target function should be set") };
                let detour = unsafe { DETOUR.expect("detour function should be set") };
                detour($($arg,)* target)
            }

            static mut HOOK: $crate::Hook<
                unsafe extern "C" fn($($arg: $ty),*) -> $ret,
                unsafe extern "C" fn($($arg: $ty),*, cb: unsafe extern "C" fn($($arg: $ty),*) -> $ret) -> $ret
            > = unsafe { $crate::Hook::new(internal, ::std::ptr::addr_of_mut!(TARGET), ::std::ptr::addr_of_mut!(DETOUR)) };

            ::std::ptr::addr_of_mut!(HOOK)
        };
    )*};
}

#[derive(Debug)]
pub struct Hook<O, R>(O, *mut Option<O>, *mut Option<R>);

impl<O, R> Hook<O, R> {
    #[inline]
    pub const fn new(original: O, cb_ref: *mut Option<O>, detour_ref: *mut Option<R>) -> Self {
        Self(original, cb_ref, detour_ref)
    }
}

pub trait FnPtr<Args, Ret> {
    fn to_ptr(&self) -> VoidPtr;
}

macro_rules! impl_fn_ptr {
    ($($ty:ident),*) => {
        impl <$($ty,)* Ret> FnPtr<($($ty,)*), Ret> for unsafe extern "C" fn($($ty,)*) -> Ret {
            #[inline]
            fn to_ptr(&self) -> VoidPtr {
                *self as _
            }
        }
    }
}

impl_fn_ptr!();
impl_fn_ptr!(A);
impl_fn_ptr!(A, B);
impl_fn_ptr!(A, B, C);
impl_fn_ptr!(A, B, C, D);
impl_fn_ptr!(A, B, C, D, E);
impl_fn_ptr!(A, B, C, D, E, F);
impl_fn_ptr!(A, B, C, D, E, F, G);
impl_fn_ptr!(A, B, C, D, E, F, G, H);

pub trait Plugin<Env: From<SdkEnv> = SdkEnv> {
    const NAME: &'static U16CStr;
    const AUTHOR: &'static U16CStr;
    const VERSION: SemVer;
    const SDK: SdkVersion = SdkVersion::LATEST;
    const RUNTIME: RuntimeVersion = RuntimeVersion::RUNTIME_INDEPENDENT;
    const API_VERSION: ApiVersion = ApiVersion::LATEST;

    fn on_init(env: &Env);
}

pub trait PluginSyntax<Env: From<SdkEnv>>: Plugin<Env> {
    fn env() -> &'static Env;
    fn env_lock() -> &'static OnceLock<Box<dyn std::any::Any + Send + Sync>>;
    fn plugin_info() -> red::PluginInfo;
}

impl<P, Env> PluginSyntax<Env> for P
where
    Env: From<SdkEnv>,
    P: Plugin<Env>,
{
    fn env() -> &'static Env {
        Self::env_lock()
            .get()
            .expect("plugin environment should be initialized")
            .downcast_ref()
            .unwrap()
    }

    #[inline]
    fn env_lock() -> &'static OnceLock<Box<dyn std::any::Any + Send + Sync>> {
        static ENV: OnceLock<Box<dyn std::any::Any + Send + Sync>> = OnceLock::new();
        &ENV
    }

    fn plugin_info() -> red::PluginInfo {
        red::PluginInfo {
            name: Self::NAME.as_ptr(),
            author: Self::AUTHOR.as_ptr(),
            version: Self::VERSION.0,
            runtime: Self::RUNTIME.0,
            sdk: Self::SDK.0 .0,
        }
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($trait:ty) => {
        mod __api {
            use super::*;

            #[no_mangle]
            #[allow(non_snake_case, unused_variables)]
            unsafe extern "C" fn Query(info: *mut $crate::internal::PluginInfo) {
                *info = <$trait as $crate::PluginSyntax<_>>::plugin_info();
            }

            #[no_mangle]
            #[allow(non_snake_case, unused_variables)]
            extern "C" fn Main(
                handle: $crate::internal::PluginHandle,
                reason: $crate::internal::EMainReason::Type,
                sdk: $crate::internal::Sdk,
            ) {
                let lock = <$trait as $crate::PluginSyntax<_>>::env_lock();
                lock.set(Box::new($crate::SdkEnv::new(handle, sdk)))
                    .expect("plugin environment should be initialized");
                <$trait as $crate::Plugin<_>>::on_init(<$trait as $crate::PluginSyntax<_>>::env());
            }

            #[no_mangle]
            #[allow(non_snake_case, unused_variables)]
            extern "C" fn Supports() -> u32 {
                <$trait as $crate::Plugin<_>>::API_VERSION.into()
            }
        }
    };
}

pub type OpcodeHandler = unsafe extern "C" fn(
    ctx: *mut IScriptable,
    frame: *mut StackFrame,
    arg3: VoidPtr,
    arg4: VoidPtr,
);

pub type StateHandler = unsafe extern "C" fn(app: &GameApp);

pub type VoidPtr = *mut std::os::raw::c_void;

#[repr(transparent)]
pub struct GameApp(red::CGameApplication);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct StateListener(red::GameState);

#[allow(clippy::missing_transmute_annotations)]
impl StateListener {
    #[inline]
    pub fn with_on_enter(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnEnter: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    #[inline]
    pub fn with_on_update(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnUpdate: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    #[inline]
    pub fn with_on_exit(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnExit: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ValueContainer(VoidPtr);

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ValuePtr(VoidPtr);

impl ValuePtr {
    pub unsafe fn unwrap_ref(&self) -> Option<&IScriptable> {
        let ptr = self.0 as *mut red::SharedPtrBase<red::IScriptable>;
        let inst = (*ptr).instance;
        let rc = (*ptr).refCount;
        if inst.is_null() || rc.is_null() || (*rc).strongRefs == 0 {
            return None;
        };
        Some(&*(inst as *const IScriptable))
    }

    pub unsafe fn unwrap_script_ref(&self) -> Option<ValuePtr> {
        let ptr = &*(self.0 as *mut ScriptRefAny);
        if !ptr.is_defined() {
            return None;
        };
        Some(ptr.value())
    }

    #[inline]
    pub unsafe fn to_container(&self) -> ValueContainer {
        ValueContainer(self.0)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct IScriptable(red::IScriptable);

impl IScriptable {
    pub fn class(&self) -> &'static Class {
        unsafe {
            &*(((*self.0._base.vtable_).ISerializable_GetType)((&self.0._base) as *const _ as _)
                as *const Class)
        }
    }

    #[inline]
    pub fn fields(&self) -> ValueContainer {
        ValueContainer(unsafe { red::IScriptable_GetValueHolder(&self.0 as *const _ as *mut _) })
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct StackFrame(red::CStackFrame);

impl StackFrame {
    #[inline]
    pub fn func(&self) -> &Function {
        unsafe { &*(self.0.func as *const Function) }
    }

    #[inline]
    pub fn parent(&self) -> Option<&StackFrame> {
        unsafe { (self.0.parent as *const StackFrame).as_ref() }
    }

    #[inline]
    pub fn parent_iter(&self) -> impl Iterator<Item = &StackFrame> {
        iter::successors(self.parent(), |frame| frame.parent())
    }

    #[inline]
    pub fn context(&self) -> Option<&IScriptable> {
        unsafe { (self.0.context as *const IScriptable).as_ref() }
    }

    #[inline]
    pub fn has_code(&self) -> bool {
        !self.0.code.is_null()
    }

    pub unsafe fn instr_at<A: Instr>(&self, offset: isize) -> Option<&A> {
        if self.0.code.is_null() {
            return None;
        }
        let ptr = self.0.code.offset(offset);
        (ptr.read() as u8 == A::OPCODE).then(|| &*(ptr.offset(OPCODE_SIZE) as *const A))
    }

    #[inline]
    pub fn locals(&self) -> ValueContainer {
        ValueContainer(self.0.localVars)
    }

    #[inline]
    pub fn params(&self) -> ValueContainer {
        ValueContainer(self.0.params)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Function(red::CBaseFunction);

impl Function {
    #[inline]
    pub fn name(&self) -> CName {
        CName(self.0.fullName)
    }

    #[inline]
    pub fn parent(&self) -> Option<&Class> {
        unsafe { (self.vft().get_parent)(self).as_ref() }
    }

    #[inline]
    pub fn locals(&self) -> &Array<&Property> {
        unsafe { mem::transmute(&self.0.localVars) }
    }

    #[inline]
    pub fn params(&self) -> &Array<&Property> {
        unsafe { mem::transmute(&self.0.params) }
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        self.0.flags.isStatic() != 0
    }

    #[inline]
    fn vft(&self) -> &FunctionVft {
        unsafe { &*(self.0._base.vtable_ as *const FunctionVft) }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct CName(red::CName);

impl CName {
    #[inline]
    pub const fn undefined() -> Self {
        Self(red::CName { hash: 0 })
    }

    #[inline]
    pub const fn new(name: &str) -> Self {
        Self(red::CName {
            hash: fnv1a64(name),
        })
    }

    pub fn as_str(&self) -> &'static str {
        unsafe { ffi::CStr::from_ptr(self.0.ToString()) }
            .to_str()
            .unwrap()
    }
}

impl PartialEq for CName {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for CName {}

impl PartialOrd for CName {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CName {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.hash.cmp(&other.0.hash)
    }
}

impl Hash for CName {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Class(red::CClass);

impl Class {
    #[inline]
    pub fn name(&self) -> CName {
        CName(self.0.name)
    }

    #[inline]
    pub fn properties(&self) -> &Array<&Property> {
        unsafe { mem::transmute(&self.0.props) }
    }

    #[inline]
    pub fn base(&self) -> Option<&Class> {
        unsafe { (self.0.parent as *const Class).as_ref() }
    }

    #[inline]
    pub fn base_iter(&self) -> impl Iterator<Item = &Class> {
        iter::successors(self.base(), |class| class.base())
    }

    pub fn all_properties(&self) -> impl Iterator<Item = &Property> {
        iter::once(self)
            .chain(self.base_iter())
            .flat_map(Class::properties)
            .copied()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ArrayType(red::CRTTIBaseArrayType);

impl ArrayType {
    #[inline]
    pub fn inner_type(&self) -> &'static Type {
        unsafe { &*(self.vft().get_inner_type)(self) }
    }

    #[inline]
    pub unsafe fn length(&self, val: ValuePtr) -> u32 {
        unsafe { (self.vft().get_length)(self, val) }
    }

    #[inline]
    pub unsafe fn element(&self, val: ValuePtr, index: u32) -> ValuePtr {
        unsafe { (self.vft().get_element)(self, val, index) }
    }

    #[inline]
    fn vft(&self) -> &ArrayTypeVft {
        unsafe { &*(self.0._base.vtable_ as *const ArrayTypeVft) }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Array<T>(red::DynArray<T>);

impl<T> ops::Deref for Array<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        if self.0.entries.is_null() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.0.entries, self.0.size as usize) }
        }
    }
}

impl<'a, T> IntoIterator for &'a Array<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[repr(transparent)]
pub struct String(red::CString);

impl String {
    #[inline]
    pub fn new() -> Self {
        Self(unsafe { red::CString::new(ptr::null_mut()) })
    }
}

impl Default for String {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        unsafe { ffi::CStr::from_ptr(self.0.c_str()) }
            .to_str()
            .unwrap()
    }
}

impl fmt::Display for String {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ScriptRef<A>(red::ScriptRef<A>);

impl<A> ScriptRef<A> {
    #[inline]
    pub fn inner_type(&self) -> &Type {
        unsafe { &*(self.0.innerType as *const Type) }
    }

    #[inline]
    pub fn is_defined(&self) -> bool {
        !self.0.ref_.is_null()
    }
}

pub type ScriptRefAny = ScriptRef<std::os::raw::c_void>;

impl ScriptRefAny {
    #[inline]
    pub fn value(&self) -> ValuePtr {
        ValuePtr(self.0.ref_)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Property(red::CProperty);

impl Property {
    #[inline]
    pub fn name(&self) -> CName {
        CName(self.0.name)
    }

    #[inline]
    pub fn type_(&self) -> &'static Type {
        unsafe { &*(self.0.type_ as *const Type) }
    }

    #[inline]
    pub unsafe fn value(&self, container: ValueContainer) -> ValuePtr {
        unsafe { ValuePtr(container.0.byte_add(self.0.valueOffset as usize)) }
    }

    #[inline]
    pub fn is_in_value_holder(&self) -> bool {
        self.0.flags.inValueHolder() != 0
    }

    #[inline]
    pub fn is_scripted(&self) -> bool {
        self.0.flags.isScripted() != 0
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Type(red::CBaseRTTIType);

impl Type {
    #[inline]
    pub fn name(&self) -> CName {
        CName(unsafe { (self.vft().tail.CBaseRTTIType_GetName)(&self.0) })
    }

    #[inline]
    pub fn size(&self) -> u32 {
        unsafe { (self.vft().tail.CBaseRTTIType_GetSize)(&self.0) }
    }

    #[inline]
    pub fn alignment(&self) -> u32 {
        unsafe { (self.vft().tail.CBaseRTTIType_GetAlignment)(&self.0) }
    }

    #[inline]
    pub fn kind(&self) -> Kind {
        unsafe { mem::transmute((self.vft().tail.CBaseRTTIType_GetType)(&self.0)) }
    }

    #[inline]
    pub fn as_class(&self) -> Option<&Class> {
        if self.kind().is_class() {
            Some(unsafe { mem::transmute::<&red::CBaseRTTIType, &Class>(&self.0) })
        } else {
            None
        }
    }

    #[inline]
    pub fn as_array(&self) -> Option<&ArrayType> {
        if self.kind().is_array() {
            Some(unsafe { mem::transmute::<&red::CBaseRTTIType, &ArrayType>(&self.0) })
        } else {
            None
        }
    }

    pub unsafe fn to_string(&self, value: ValuePtr) -> String {
        let mut str = String::new();
        unsafe { (self.vft().tail.CBaseRTTIType_ToString)(&self.0, value.0, &mut str.0) };
        str
    }

    #[inline]
    fn vft(&self) -> &TypeVft {
        unsafe { &*(self.0.vtable_ as *const TypeVft) }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Name = red::ERTTIType::Name,
    Fundamental = red::ERTTIType::Fundamental,
    Class = red::ERTTIType::Class,
    Array = red::ERTTIType::Array,
    Simple = red::ERTTIType::Simple,
    Enum = red::ERTTIType::Enum,
    StaticArray = red::ERTTIType::StaticArray,
    NativeArray = red::ERTTIType::NativeArray,
    Pointer = red::ERTTIType::Pointer,
    Handle = red::ERTTIType::Handle,
    WeakHandle = red::ERTTIType::WeakHandle,
    ResourceReference = red::ERTTIType::ResourceReference,
    ResourceAsyncReference = red::ERTTIType::ResourceAsyncReference,
    BitField = red::ERTTIType::BitField,
    LegacySingleChannelCurve = red::ERTTIType::LegacySingleChannelCurve,
    ScriptReference = red::ERTTIType::ScriptReference,
    FixedArray = red::ERTTIType::FixedArray,
}

impl Kind {
    #[inline]
    pub fn is_pointer(self) -> bool {
        matches!(self, Self::Pointer | Self::Handle | Self::WeakHandle)
    }

    #[inline]
    pub fn is_class(self) -> bool {
        self == Self::Class
    }

    #[inline]
    pub fn is_array(self) -> bool {
        matches!(
            self,
            Self::Array | Self::StaticArray | Self::NativeArray | Self::FixedArray
        )
    }
}

#[derive(Debug)]
#[repr(u32)]
pub enum StateType {
    BaseInitialization = red::EGameStateType::BaseInitialization,
    Initialization = red::EGameStateType::Initialization,
    Running = red::EGameStateType::Running,
    Shutdown = red::EGameStateType::Shutdown,
}

pub const OPCODE_SIZE: isize = 1;
pub const CALL_INSTR_SIZE: isize = mem::size_of::<InvokeStatic>() as isize;

pub trait Instr {
    const OPCODE: u8;
}

#[derive(Debug)]
#[repr(packed)]
pub struct InvokeStatic {
    pub skip: u16,
    pub line: u16,
    pub func: *mut Function,
    pub flags: u16,
}

impl Instr for InvokeStatic {
    const OPCODE: u8 = 36;
}

#[derive(Debug)]
#[repr(packed)]
pub struct InvokeVirtual {
    pub skip: u16,
    pub line: u16,
    pub name: CName,
    pub flags: u16,
}

impl Instr for InvokeVirtual {
    const OPCODE: u8 = 37;
}

#[repr(C)]
struct TypeVft {
    destroy: unsafe extern "fastcall" fn(this: *mut Type),
    tail: red::CBaseRTTIType__bindgen_vtable,
}

#[repr(C)]
struct ArrayTypeVft {
    base: TypeVft,
    get_inner_type: unsafe extern "fastcall" fn(this: *const ArrayType) -> *const Type,
    sub_c8: unsafe extern "fastcall" fn(this: *const ArrayType) -> bool,
    get_length: unsafe extern "fastcall" fn(this: *const ArrayType, val: ValuePtr) -> u32,
    get_max_length: unsafe extern "fastcall" fn(this: *const ArrayType) -> u32,
    get_element:
        unsafe extern "fastcall" fn(this: *const ArrayType, val: ValuePtr, index: u32) -> ValuePtr,
}

#[repr(C)]
struct FunctionVft {
    get_allocator: unsafe extern "fastcall" fn(this: &Function) -> *mut red::Memory::Allocator,
    destroy: unsafe extern "fastcall" fn(this: &Function),
    get_parent: unsafe extern "fastcall" fn(this: &Function) -> *const Class,
}

const fn fnv1a64(str: &str) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    const SEED: u64 = 0xCBF2_9CE4_8422_2325;

    let mut tail = str.as_bytes();
    let mut hash = SEED;
    loop {
        match tail.split_first() {
            Some((head, rem)) => {
                hash ^= *head as u64;
                hash = hash.wrapping_mul(PRIME);
                tail = rem;
            }
            None => break hash,
        }
    }
}

fn truncated_cstring(mut s: std::string::String) -> ffi::CString {
    s.truncate(s.find('\0').unwrap_or(s.len()));
    ffi::CString::new(s).unwrap()
}
