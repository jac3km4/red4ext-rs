#![doc = include_str!("../README.md")]
#![allow(clippy::missing_safety_doc)]
use std::ffi::CString;
use std::sync::OnceLock;
use std::{ffi, fmt, mem};

pub use export::{
    ClassExport, ClassExportBuilder, ExportList, ExportNil, Exportable, GlobalExport, StructExport,
    StructExportBuilder,
};
use raw::root::{RED4ext as red, versioning};
use sealed::sealed;
use types::StaticArray;
pub use widestring::{U16CStr, widecstr as wcstr};

mod class;
mod export;
mod invocable;
mod raw;
mod repr;
mod systems;

/// Rust wrappers for [SDK types](https://github.com/WopsS/RED4ext.SDK/tree/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext).
/// Engine-backed allocation, RTTI, and singleton access require a live game runtime.
pub mod types;

pub use class::{ClassKind, ScriptClass, ScriptClassOps, class_kind};
pub use invocable::{
    AsReceiver, FunctionType, GlobalInvocable, GlobalMetadata, InvokeError, MethodInvocable,
    MethodMetadata, SignatureError,
};
pub use repr::{FromRepr, IntoRepr, NativeRepr};
pub use systems::{RttiRegistrator, RttiSystem, RttiSystemMut};

/// SDK [address hashes](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Detail/AddressHashes.hpp)
/// for runtime relocation. Resolve only inside a running game.
///
/// # Example
/// Resolve a hash to an address:
/// ```rust
/// use red4ext_rs::addr_hashes;
///
/// fn exec_hash() -> usize {
///     addr_hashes::resolve(addr_hashes::CBaseFunction_ExecuteNative)
/// }
/// ```
pub mod addr_hashes {
    pub use super::red::Detail::AddressHashes::*;

    /// Resolves a hash to an address.
    #[inline]
    pub fn resolve(hash: u32) -> usize {
        unsafe { super::red::UniversalRelocBase::Resolve(hash) }
    }
}

#[doc(hidden)]
pub mod internal {
    pub use crate::red::v1::{EMainReason, PluginHandle, PluginInfo, Sdk};
}

#[doc(hidden)]
pub type VoidPtr = *mut std::os::raw::c_void;

/// Plugin metadata, exports, and load/unload callbacks.
/// See SDK [PluginInfo](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/PluginInfo.hpp)
/// and export the entry points with [`export_plugin_symbols!`].
pub trait Plugin {
    /// The name of the plugin.
    const NAME: &'static U16CStr;
    /// The author of the plugin.
    const AUTHOR: &'static U16CStr;
    /// The version of the plugin.
    const VERSION: SemVer;
    /// The RED4ext SDK version the plugin was built with.
    const SDK: SdkVersion = SdkVersion::LATEST;
    /// Loader runtime requirement. Runtime-independent does not guarantee ABI
    /// compatibility across game updates.
    const RUNTIME: RuntimeVersion = RuntimeVersion::RUNTIME_INDEPENDENT;
    /// The RED4ext API version.
    const API_VERSION: ApiVersion = ApiVersion::LATEST;

    /// Definitions registered during the SDK's RTTI registration callbacks.
    /// Use [`exports!`] to collect classes and functions.
    fn exports() -> impl Exportable {
        ExportNil
    }

    /// Called for SDK [Load](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/EMainReason.hpp).
    /// Game systems and RTTI registration may not yet be ready.
    fn on_load(_env: &SdkEnv) {}
    /// Called for SDK [Unload](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/EMainReason.hpp).
    /// Release plugin-owned runtime resources before returning.
    fn on_unload(_env: &SdkEnv) {}
}

/// Result of a [`StateHandler`]; see SDK
/// [GameState](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/GameState.hpp).
/// Only update callbacks outside `Running` use the result to control repetition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum StateHandlerResult {
    /// Continue updating on subsequent frames.
    Running = 0,
    /// Stop updating. Return this for enter and exit callbacks.
    Finished = 1,
}

/// A set of useful operations that can be performed on a plugin.
#[sealed]
pub trait PluginOps: Plugin {
    /// Retrieves a statically initialized reference to the plugin environment.
    /// It can be used to log messages, add state listeners, and attach hooks.
    fn env() -> &'static SdkEnv;

    #[doc(hidden)]
    fn env_lock() -> &'static OnceLock<Box<SdkEnv>>;
    #[doc(hidden)]
    fn info() -> PluginInfo;
    #[doc(hidden)]
    fn load(env: SdkEnv);
    #[doc(hidden)]
    fn unload(env: SdkEnv);
}

#[sealed]
impl<P> PluginOps for P
where
    P: Plugin,
{
    fn env() -> &'static SdkEnv {
        Self::env_lock().get().unwrap()
    }

    #[inline]
    fn env_lock() -> &'static OnceLock<Box<SdkEnv>> {
        static ENV: OnceLock<Box<SdkEnv>> = OnceLock::new();
        &ENV
    }

    #[inline]
    fn info() -> PluginInfo {
        PluginInfo::new(
            Self::NAME,
            Self::AUTHOR,
            Self::SDK,
            Self::VERSION,
            Self::RUNTIME,
        )
    }

    fn load(env: SdkEnv) {
        Self::env_lock()
            .set(Box::new(env))
            .expect("plugin environment should not be initialized");

        #[cfg(feature = "log")]
        {
            log::set_logger(Self::env()).unwrap();
            log::set_max_level(log::LevelFilter::Trace);
        }

        Self::on_load(Self::env());
    }

    fn unload(env: SdkEnv) {
        Self::on_unload(&env);
    }
}

/// Exports `Query`, `Main`, and `Supports` for a [`Plugin`].
/// Required once per plugin DLL; see the SDK
/// [plugin example](https://github.com/WopsS/RED4ext.SDK/tree/2216befc23924b46996a9335043aebb31ba6a757/examples).
#[macro_export]
macro_rules! export_plugin_symbols {
    ($trait:ty) => {
        // The exports will not work in tests, so they are disabled here.
        #[cfg(not(test))]
        mod __api {
            use super::*;

            #[unsafe(no_mangle)]
            #[allow(non_snake_case, unused_variables)]
            unsafe extern "C" fn Query(info: *mut $crate::internal::PluginInfo) {
                *info = <$trait as $crate::PluginOps>::info().into_raw();
            }

            #[unsafe(no_mangle)]
            #[allow(non_snake_case, unused_variables)]
            extern "C" fn Main(
                handle: $crate::internal::PluginHandle,
                reason: $crate::internal::EMainReason::Type,
                sdk: $crate::internal::Sdk,
            ) -> bool {
                match reason {
                    $crate::internal::EMainReason::Load => {
                        <$trait as $crate::PluginOps>::load($crate::SdkEnv::new(handle, sdk));
                        $crate::RttiRegistrator::add(Some(on_register), Some(on_post_register));
                    }
                    $crate::internal::EMainReason::Unload => {
                        <$trait as $crate::PluginOps>::unload($crate::SdkEnv::new(handle, sdk));
                    }
                    _ => {}
                }
                true
            }

            #[unsafe(no_mangle)]
            #[allow(non_snake_case, unused_variables)]
            extern "C" fn Supports() -> u32 {
                ::std::convert::Into::into(<$trait as $crate::Plugin>::API_VERSION)
            }

            extern "C" fn on_register() {
                let exports = <$trait as $crate::Plugin>::exports();
                $crate::Exportable::register(&exports);
            }

            extern "C" fn on_post_register() {
                let exports = <$trait as $crate::Plugin>::exports();
                $crate::Exportable::post_register(&exports);
            }
        }
    };
}

/// Logging through the SDK [Logger](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/Logger.hpp).
/// Without the `log` feature, pass [`SdkEnv`] first: `log::info!(env, "loaded")`.
/// With `log` enabled, this is the `log` crate: `log::info!("loaded")`.
#[cfg(not(feature = "log"))]
pub mod log {
    #[doc(inline)]
    pub use crate::{
        __debug as debug, __error as error, __info as info, __trace as trace, __warn as warn,
    };

    /// Logs an info message: `log::info!(env, "loaded")`.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __info {
        ($env:expr, $($arg:tt)*) => {
            $env.info(format_args!($($arg)*))
        };
    }

    /// Logs a warning: `log::warn!(env, "unexpected value")`.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __warn {
        ($env:expr, $($arg:tt)*) => {
            $env.warn(format_args!($($arg)*))
        };
    }

    /// Logs an error: `log::error!(env, "operation failed")`.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __error {
        ($env:expr, $($arg:tt)*) => {
            $env.error(format_args!($($arg)*))
        };
    }

    /// Logs a debug message: `log::debug!(env, "value: {}", 42)`.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __debug {
        ($env:expr, $($arg:tt)*) => {
            $env.debug(format_args!($($arg)*))
        };
    }

    /// Logs a trace message: `log::trace!(env, "entered callback")`.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __trace {
        ($env:expr, $($arg:tt)*) => {
            $env.trace(format_args!($($arg)*))
        };
    }
}

#[cfg(feature = "log")]
pub use log;

macro_rules! log_internal {
    ($self:ident, $level:ident, $msg:expr) => {
        unsafe {
            let str = truncated_cstring($msg.to_string());
            ((*$self.sdk.logger).$level.unwrap())($self.handle, str.as_ptr());
        }
    };
}

/// Plugin-scoped SDK services: logging, hooks, and state listeners.
/// See SDK [Sdk](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/Sdk.hpp).
/// Available in lifecycle callbacks or through [`PluginOps::env`] after load.
#[derive(Debug)]
pub struct SdkEnv {
    handle: red::v1::PluginHandle,
    sdk: red::v1::Sdk,
}

impl SdkEnv {
    #[doc(hidden)]
    pub fn new(handle: red::v1::PluginHandle, sdk: red::v1::Sdk) -> Self {
        Self { handle, sdk }
    }

    /// Logs a message at the info level.
    /// You should generally use the [`info!`](crate::log::info) macro instead of calling this method directly.
    #[inline]
    pub fn info(&self, txt: impl fmt::Display) {
        log_internal!(self, Info, txt);
    }

    /// Logs a message at the warn level.
    /// You should generally use the [`warn!`](crate::log::warn) macro instead of calling this method directly.
    #[inline]
    pub fn warn(&self, txt: impl fmt::Display) {
        log_internal!(self, Warn, txt);
    }

    /// Logs a message at the error level.
    /// You should generally use the [`error!`](crate::log::error) macro instead of calling this method directly.
    #[inline]
    pub fn error(&self, txt: impl fmt::Display) {
        log_internal!(self, Error, txt);
    }

    /// Logs a message at the debug level.
    /// You should generally use the [`debug!`](crate::log::debug) macro instead of calling this method directly.
    #[inline]
    pub fn debug(&self, txt: impl fmt::Display) {
        log_internal!(self, Debug, txt);
    }

    /// Logs a message at the trace level.
    /// You should generally use the [`trace!`](crate::log::trace) macro instead of calling this method directly.
    #[inline]
    pub fn trace(&self, txt: impl fmt::Display) {
        log_internal!(self, Trace, txt);
    }

    /// Adds a listener to a specific state type.
    /// The listener will be called when the state is entered, updated, or exited.
    /// See [`StateType`] for the available state types.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{GameApp, SdkEnv, StateHandlerResult, StateListener, StateType};
    ///
    /// fn add_state_listener(env: &SdkEnv) {
    ///     let listener = StateListener::default()
    ///         .with_on_enter(on_enter)
    ///         .with_on_exit(on_exit);
    ///     env.add_listener(StateType::Running, listener);
    /// }
    ///
    /// unsafe extern "C" fn on_enter(app: &GameApp) -> StateHandlerResult {
    ///     // do something here...
    ///     StateHandlerResult::Finished
    /// }
    ///
    /// unsafe extern "C" fn on_exit(app: &GameApp) -> StateHandlerResult {
    ///     // do something here...
    ///     StateHandlerResult::Finished
    /// }
    /// ```
    #[inline]
    pub fn add_listener(&self, typ: StateType, mut listener: StateListener) -> bool {
        unsafe { ((*self.sdk.gameStates).Add.unwrap())(self.handle, typ as u32, &mut listener.0) }
    }

    /// Attaches an SDK [hook](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/Hooking.hpp).
    /// The detour's last argument is the trampoline; call it to run the original.
    /// Returns whether attachment succeeded.
    ///
    /// # Safety
    /// The target and detour functions must both be valid and compatible function pointers.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{SdkEnv, hooks};
    ///
    /// hooks! {
    ///    static ADD_HOOK: fn(a: u32, b: u32) -> u32;
    /// }
    ///
    /// fn attach_my_hook(env: &SdkEnv, addr: unsafe extern "C" fn(u32, u32) -> u32) {
    ///     unsafe { env.attach_hook(ADD_HOOK, addr, detour) };
    /// }
    ///
    /// unsafe extern "C" fn detour(a: u32, b: u32, cb: unsafe extern "C" fn(u32, u32) -> u32) -> u32 {
    ///     // do something here...
    ///     cb(a, b)
    /// }
    /// ```
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
                (*cb_ref).cast::<VoidPtr>(),
            )
        }
    }

    /// Detaches SDK hooks at the target; returns whether detachment succeeded.
    /// See SDK [Hooking::Detach](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/Hooking.hpp).
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

#[cfg(feature = "log")]
impl log::Log for SdkEnv {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        match record.level() {
            log::Level::Error => self.error(record.args()),
            log::Level::Warn => self.warn(record.args()),
            log::Level::Info => self.info(record.args()),
            log::Level::Debug => self.debug(record.args()),
            log::Level::Trace => self.trace(record.args()),
        };
    }

    fn flush(&self) {}
}

/// SDK [SemVer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/SemVer.hpp).
#[derive(Debug)]
pub struct SemVer(red::v1::SemVer);

impl SemVer {
    /// Creates a new semantic version.
    #[inline]
    pub const fn new(major: u8, minor: u16, patch: u32) -> Self {
        Self::exact(major, minor, patch, 0, 0)
    }

    /// Creates a new exact semantic version.
    #[inline]
    pub const fn exact(major: u8, minor: u16, patch: u32, type_: u32, number: u32) -> Self {
        Self(red::v1::SemVer {
            major,
            minor,
            patch,
            prerelease: red::v1::SemVer_PrereleaseInfo { type_, number },
        })
    }
}

impl From<SemVer> for StaticArray<u16, 3> {
    fn from(value: SemVer) -> Self {
        Self::from([value.0.major as u16, value.0.minor, value.0.patch as u16])
    }
}

impl From<SemVer> for StaticArray<u16, 5> {
    fn from(value: SemVer) -> Self {
        Self::from([
            value.0.major as u16,
            value.0.minor,
            value.0.patch as u16,
            value.0.prerelease.type_ as u16,
            value.0.prerelease.number as u16,
        ])
    }
}

impl IntoRepr for SemVer {
    type Repr = StaticArray<u16, 3>;

    fn into_repr(self) -> Self::Repr {
        Self::Repr::from([self.0.major as u16, self.0.minor, self.0.patch as u16])
    }
}

/// SDK [runtime version](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/Runtime.hpp) requirement.
#[derive(Debug)]
pub struct RuntimeVersion(red::v1::FileVer);

impl RuntimeVersion {
    /// Disables the loader's game-version restriction; does not ensure ABI compatibility.
    pub const RUNTIME_INDEPENDENT: Self = Self(red::v1::FileVer {
        major: versioning::RUNTIME_INDEPENDENT,
        minor: versioning::RUNTIME_INDEPENDENT,
        build: versioning::RUNTIME_INDEPENDENT,
        revision: versioning::RUNTIME_INDEPENDENT,
    });
}

/// SDK [version](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Version.hpp) used to build the plugin.
#[derive(Debug)]
pub struct SdkVersion(SemVer);

impl SdkVersion {
    /// The latest version of the RED4ext SDK compatible with this version of the library.
    pub const LATEST: Self = Self(SemVer::new(
        versioning::SDK_MAJOR,
        versioning::SDK_MINOR,
        versioning::SDK_PATCH,
    ));
}

/// Loader [API version](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/ApiVersion.hpp).
#[derive(Debug)]
pub struct ApiVersion(u32);

impl ApiVersion {
    /// The latest version of the RED4ext API compatible with this version of the library.
    pub const LATEST: Self = Self(versioning::API_VERSION_LATEST);
}

impl From<ApiVersion> for u32 {
    #[inline]
    fn from(api: ApiVersion) -> u32 {
        api.0
    }
}

/// Defines a set of hooks that can be attached to target functions.
/// The hooks are defined as static variables and must be initialized with a call to
/// [`SdkEnv::attach_hook`].
///
/// # Example
/// ```rust
/// use red4ext_rs::hooks;
///
/// hooks! {
///    static ADD_HOOK: fn(a: u32, b: u32) -> u32;
/// }
/// ```
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
                unsafe {
                    let target = TARGET.expect("target function should be set");
                    let detour = DETOUR.expect("detour function should be set");
                    detour($($arg,)* target)
                }
            }

            static mut HOOK: $crate::Hook<
                unsafe extern "C" fn($($arg: $ty),*) -> $ret,
                unsafe extern "C" fn($($arg: $ty),*, cb: unsafe extern "C" fn($($arg: $ty),*) -> $ret) -> $ret
            > = unsafe { $crate::Hook::new(internal, ::std::ptr::addr_of_mut!(TARGET), ::std::ptr::addr_of_mut!(DETOUR)) };

            ::std::ptr::addr_of_mut!(HOOK)
        };
    )*};
}

/// A wrapper around function pointers that can be passed to [`SdkEnv::attach_hook`] to install
/// detours.
#[derive(Debug)]
pub struct Hook<O, R>(O, *mut Option<O>, *mut Option<R>);

#[doc(hidden)]
impl<O, R> Hook<O, R> {
    #[inline]
    pub const fn new(original: O, cb_ref: *mut Option<O>, detour_ref: *mut Option<R>) -> Self {
        Self(original, cb_ref, detour_ref)
    }
}

/// A trait for functions that are convertible to pointers. Only non-closure functions can
/// satisfy this requirement.
#[sealed]
pub trait FnPtr<Args, Ret> {
    fn to_ptr(&self) -> VoidPtr;
}

macro_rules! impl_fn_ptr {
    ($($ty:ident),*) => {
        #[sealed]
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
impl_fn_ptr!(A, B, C, D, E, F, G, H, I);
impl_fn_ptr!(A, B, C, D, E, F, G, H, I, J);

/// A callback function to be called when a state is entered, updated, or exited.
pub type StateHandler = unsafe extern "C" fn(app: &GameApp) -> StateHandlerResult;

/// SDK [CGameApplication](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/GameApplication.hpp) passed to state callbacks.
#[repr(transparent)]
pub struct GameApp(red::CGameApplication);

/// SDK [GameState](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Api/v1/GameState.hpp)
/// callbacks. Register with [`SdkEnv::add_listener`].
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct StateListener(red::v1::GameState);

#[allow(clippy::missing_transmute_annotations)]
impl StateListener {
    /// Called when the state activates. Return [`StateHandlerResult::Finished`];
    /// the SDK ignores the result.
    #[inline]
    pub fn with_on_enter(self, cb: StateHandler) -> Self {
        Self(red::v1::GameState {
            OnEnter: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    /// Called each frame until it returns [`StateHandlerResult::Finished`].
    /// In [`StateType::Running`], the result is ignored and updates continue.
    #[inline]
    pub fn with_on_update(self, cb: StateHandler) -> Self {
        Self(red::v1::GameState {
            OnUpdate: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    /// Called when the state ends. Return [`StateHandlerResult::Finished`];
    /// the SDK ignores the result.
    #[inline]
    pub fn with_on_exit(self, cb: StateHandler) -> Self {
        Self(red::v1::GameState {
            OnExit: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }
}

/// SDK [game state types](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/GameStates.hpp).
#[derive(Debug)]
#[repr(u32)]
pub enum StateType {
    BaseInitialization = red::EGameStateType::BaseInitialization,
    Initialization = red::EGameStateType::Initialization,
    Running = red::EGameStateType::Running,
    Shutdown = red::EGameStateType::Shutdown,
}

/// Information about a plugin.
#[derive(Debug)]
#[doc(hidden)]
#[repr(transparent)]
pub struct PluginInfo(red::v1::PluginInfo);

impl PluginInfo {
    #[doc(hidden)]
    #[inline]
    pub const fn new(
        name: &'static U16CStr,
        author: &'static U16CStr,
        sdk: SdkVersion,
        version: SemVer,
        runtime: RuntimeVersion,
    ) -> Self {
        Self(red::v1::PluginInfo {
            name: name.as_ptr(),
            author: author.as_ptr(),
            sdk: sdk.0.0,
            version: version.0,
            runtime: runtime.0,
        })
    }

    #[doc(hidden)]
    #[inline]
    pub fn into_raw(self) -> red::v1::PluginInfo {
        self.0
    }
}

const fn fnv1a64(bytes: &[u8]) -> u64 {
    const SEED: u64 = 0xCBF2_9CE4_8422_2325;

    let mut tail = bytes;
    let mut hash = SEED;
    while let [head, rem @ ..] = tail {
        tail = rem;
        hash = fnv1a64_step(hash, *head);
    }
    hash
}

const fn fnv1a64_step(hash: u64, byte: u8) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    (hash ^ byte as u64).wrapping_mul(PRIME)
}

const fn fnv1a32(str: &str) -> u32 {
    const PRIME: u32 = 0x0100_0193;
    const SEED: u32 = 0x811C_9DC5;

    let mut tail = str.as_bytes();
    let mut hash = SEED;
    loop {
        match tail.split_first() {
            Some((head, rem)) => {
                hash ^= *head as u32;
                hash = hash.wrapping_mul(PRIME);
                tail = rem;
            }
            None => break hash,
        }
    }
}

fn truncated_cstring(mut s: String) -> ffi::CString {
    s.truncate(s.find('\0').unwrap_or(s.len()));
    unsafe { CString::from_vec_unchecked(s.into_bytes()) }
}

macro_rules! fn_from_hash {
    ($name:ident, $ty:ty) => {{
        $crate::fn_from_hash!($name, $ty, offset: 0)
    }};
    ($name:ident, $ty:ty, offset: $offset:expr) => {{
        unsafe fn inner() -> $ty {
            static STATIC: ::once_cell::race::OnceNonZeroUsize =
                ::once_cell::race::OnceNonZeroUsize::new();
            let addr = STATIC
                .get_or_try_init(|| {
                    ::std::num::NonZero::new($crate::addr_hashes::resolve($crate::addr_hashes::$name)).ok_or(())
                })
                .expect(::std::stringify!(should resolve $name hash))
                .get();
            unsafe {
                ::std::mem::transmute::<usize, $ty>(addr + $offset)
            }
        }
        inner()
    }};
}

pub(crate) use fn_from_hash;

#[cold]
#[track_caller]
fn check_invariant(success: bool, message: &'static str) {
    #[cfg(feature = "log")]
    if !success {
        log::error!(
            "invariant violated: {message}: {}",
            std::panic::Location::caller(),
        );
    }
    assert!(success, "{message}");
}
