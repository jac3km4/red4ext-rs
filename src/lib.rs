#![doc = include_str!("../README.md")]
#![allow(clippy::missing_safety_doc)]
use std::ffi::CString;
use std::sync::OnceLock;
use std::{ffi, fmt, mem};

pub use export::{
    ClassExport, ClassExportBuilder, ExportList, ExportNil, Exportable, GlobalExport, StructExport,
    StructExportBuilder,
};
use raw::root::{versioning, RED4ext as red};
use sealed::sealed;
use types::StaticArray;
pub use widestring::{widecstr as wcstr, U16CStr};

mod class;
mod export;
mod invocable;
mod raw;
mod repr;
mod systems;

/// A module encapsulating various types defined in the RED4ext SDK.
pub mod types;

pub use class::{class_kind, ClassKind, ScriptClass, ScriptClassOps};
pub use invocable::{
    AsReceiver, FunctionType, GlobalInvocable, GlobalMetadata, InvokeError, MethodInvocable,
    MethodMetadata,
};
pub use repr::{FromRepr, IntoRepr, NativeRepr};
pub use systems::{RttiRegistrator, RttiSystem, RttiSystemMut};

/// Hashes of known function addresses.
///
/// # Example
/// Resolve a hash to an address:
/// ```rust
/// use red4ext_rs::addr_hashes;
///
/// fn exec_hash() -> usize {
///   addr_hashes::resolve(addr_hashes::CBaseFunction_ExecuteNative)
/// }
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
    pub use crate::red::{EMainReason, PluginHandle, PluginInfo, Sdk};
}

#[doc(hidden)]
pub type VoidPtr = *mut std::os::raw::c_void;

/// A definition of a RED4ext plugin.
pub trait Plugin {
    /// The name of the plugin.
    const NAME: &'static U16CStr;
    /// The author of the plugin.
    const AUTHOR: &'static U16CStr;
    /// The version of the plugin.
    const VERSION: SemVer;
    /// The RED4ext SDK version the plugin was built with.
    const SDK: SdkVersion = SdkVersion::LATEST;
    /// The version of the game the plugin is compatible with.
    const RUNTIME: RuntimeVersion = RuntimeVersion::RUNTIME_INDEPENDENT;
    /// The RED4ext API version.
    const API_VERSION: ApiVersion = ApiVersion::LATEST;

    /// A list of definitions to be exported automatically when the plugin is loaded.
    /// This can be used to define classes and functions that will available to use in the game.
    /// See the [`exports!`] macro for more information.
    fn exports() -> impl Exportable {
        ExportNil
    }

    /// A function that is called when the plugin is initialized.
    fn on_init(_env: &SdkEnv) {}
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
    fn init(env: SdkEnv);
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

    fn init(env: SdkEnv) {
        Self::env_lock()
            .set(Box::new(env))
            .expect("plugin environment should not be initialized");

        #[cfg(feature = "log")]
        {
            log::set_logger(Self::env()).unwrap();
            log::set_max_level(log::LevelFilter::Trace);
        }

        Self::on_init(Self::env());
    }
}

/// Defines a set of DLL symbols necessary for RED4ext to load the plugin. Your plugin will
/// not be loaded unless you call this macro.
#[macro_export]
macro_rules! export_plugin_symbols {
    ($trait:ty) => {
        // The exports will not work in tests, so they are disabled here.
        #[cfg(not(test))]
        mod __api {
            use super::*;

            #[no_mangle]
            #[allow(non_snake_case, unused_variables)]
            unsafe extern "C" fn Query(info: *mut $crate::internal::PluginInfo) {
                *info = <$trait as $crate::PluginOps>::info().into_raw();
            }

            #[no_mangle]
            #[allow(non_snake_case, unused_variables)]
            extern "C" fn Main(
                handle: $crate::internal::PluginHandle,
                reason: $crate::internal::EMainReason::Type,
                sdk: $crate::internal::Sdk,
            ) {
                if reason == $crate::internal::EMainReason::Load {
                    <$trait as $crate::PluginOps>::init($crate::SdkEnv::new(handle, sdk));
                    $crate::RttiRegistrator::add(Some(on_register), Some(on_post_register));
                }
            }

            #[no_mangle]
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

/// Convenience logging macros. By default all macros require a [`SdkEnv`] instance to be passed as
/// the first argument. If the `log` feature is enabled, this module becomes an alias to the
/// `log` crate.
#[cfg(not(feature = "log"))]
pub mod log {
    #[doc(inline)]
    pub use crate::{
        __debug as debug, __error as error, __info as info, __trace as trace, __warn as warn,
    };

    /// Logs a message at the info level.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{log, SdkEnv};
    ///
    /// fn log_info(env: &SdkEnv) {
    ///     log::info!(env, "Hello, world!");
    ///     // log::info!("Hello, world!"); // if the `log` feature is enabled
    /// }
    /// ```
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __info {
        ($env:expr, $($arg:tt)*) => {
            $env.info(format_args!($($arg)*))
        };
    }

    /// Logs a message at the warn level.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{log, SdkEnv};
    ///
    /// fn log_warn(env: &SdkEnv) {
    ///     log::warn!(env, "Hello, world!");
    ///     // log::warn!("Hello, world!"); // if the `log` feature is enabled
    /// }
    /// ```
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __warn {
        ($env:expr, $($arg:tt)*) => {
            $env.warn(format_args!($($arg)*))
        };
    }

    /// Logs a message at the error level.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{log, SdkEnv};
    ///
    /// fn log_error(env: &SdkEnv) {
    ///     log::error!(env, "Hello, world!");
    ///     // log::error!("Hello, world!"); // if the `log` feature is enabled
    /// }
    /// ```
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __error {
        ($env:expr, $($arg:tt)*) => {
            $env.error(format_args!($($arg)*))
        };
    }

    /// Logs a message at the debug level.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{log, SdkEnv};
    ///
    /// fn log_debug(env: &SdkEnv) {
    ///     log::debug!(env, "Hello, world!");
    ///     // log::debug!("Hello, world!"); // if the `log` feature is enabled
    /// }
    /// ```
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __debug {
        ($env:expr, $($arg:tt)*) => {
            $env.debug(format_args!($($arg)*))
        };
    }

    /// Logs a message at the trace level.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{log, SdkEnv};
    ///
    /// fn log_trace(env: &SdkEnv) {
    ///     log::trace!(env, "Hello, world!");
    ///     // log::trace!("Hello, world!"); // if the `log` feature is enabled
    /// }
    /// ```
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

/// A handle to the RED4ext SDK environment.
/// This struct enables access to the SDK's functions and logging facilities.
/// It can be obtained statically using the [`PluginOps::env`] method from any plugin
/// implementation.
#[derive(Debug)]
pub struct SdkEnv {
    handle: red::PluginHandle,
    sdk: red::Sdk,
}

impl SdkEnv {
    #[doc(hidden)]
    pub fn new(handle: red::PluginHandle, sdk: red::Sdk) -> Self {
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
    /// use red4ext_rs::{GameApp, SdkEnv, StateListener, StateType};
    ///
    /// fn add_state_listener(env: &SdkEnv) {
    ///     let listener = StateListener::default()
    ///         .with_on_enter(on_enter)
    ///         .with_on_exit(on_exit);
    ///     env.add_listener(StateType::Running, listener);
    /// }
    ///
    /// unsafe extern "C" fn on_enter(app: &GameApp) {
    ///     // do something here...
    /// }
    ///
    /// unsafe extern "C" fn on_exit(app: &GameApp) {
    ///     // do something here...
    /// }
    /// ```
    #[inline]
    pub fn add_listener(&self, typ: StateType, mut listener: StateListener) -> bool {
        unsafe { ((*self.sdk.gameStates).Add.unwrap())(self.handle, typ as u32, &mut listener.0) }
    }

    /// Attaches a hook to a target function.
    /// The hook will be called instead of the target function. The hook must accept a callback
    /// function as its last argument, which should be called to execute the original function.
    ///
    /// # Safety
    /// The target and detour functions must both be valid and compatible function pointers.
    ///
    /// # Example
    /// ```rust
    /// use red4ext_rs::{hooks, SdkEnv};
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

    /// Detaches a hook from a target function.
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

/// A version number in the semantic versioning format.
#[derive(Debug)]
pub struct SemVer(red::SemVer);

impl SemVer {
    /// Creates a new semantic version.
    #[inline]
    pub const fn new(major: u8, minor: u16, patch: u32) -> Self {
        Self::exact(major, minor, patch, 0, 0)
    }

    /// Creates a new exact semantic version.
    #[inline]
    pub const fn exact(major: u8, minor: u16, patch: u32, type_: u32, number: u32) -> Self {
        Self(red::SemVer {
            major,
            minor,
            patch,
            prerelease: red::v0::SemVer_PrereleaseInfo { type_, number },
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

/// A version number representing the game's version.
#[derive(Debug)]
pub struct RuntimeVersion(red::FileVer);

impl RuntimeVersion {
    /// A special version number that indicates the plugin is compatible with any game version.
    pub const RUNTIME_INDEPENDENT: Self = Self(red::FileVer {
        major: versioning::RUNTIME_INDEPENDENT,
        minor: versioning::RUNTIME_INDEPENDENT,
        build: versioning::RUNTIME_INDEPENDENT,
        revision: versioning::RUNTIME_INDEPENDENT,
    });
}

/// A version number representing the RED4ext SDK version.
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

/// A version number representing the RED4ext API version.
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

/// A callback function to be called when a state is entered, updated, or exited.
pub type StateHandler = unsafe extern "C" fn(app: &GameApp);

/// A wrapper around the game application instance.
#[repr(transparent)]
pub struct GameApp(red::CGameApplication);

/// A listener for state changes in the game application.
/// The listener can be attached to a specific state type using the [`SdkEnv::add_listener`]
/// method.
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct StateListener(red::GameState);

#[allow(clippy::missing_transmute_annotations)]
impl StateListener {
    /// Sets a callback to be called when the state is entered.
    #[inline]
    pub fn with_on_enter(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnEnter: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    /// Sets a callback to be called when the state is updated.
    #[inline]
    pub fn with_on_update(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnUpdate: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }

    /// Sets a callback to be called when the state is exited.
    #[inline]
    pub fn with_on_exit(self, cb: StateHandler) -> Self {
        Self(red::GameState {
            OnExit: Some(unsafe { mem::transmute(cb) }),
            ..self.0
        })
    }
}

/// An enum representing different types of game states.
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
pub struct PluginInfo(red::PluginInfo);

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
        Self(red::PluginInfo {
            name: name.as_ptr(),
            author: author.as_ptr(),
            sdk: sdk.0 .0,
            version: version.0,
            runtime: runtime.0,
        })
    }

    #[doc(hidden)]
    #[inline]
    pub fn into_raw(self) -> red::PluginInfo {
        self.0
    }
}

const fn fnv1a64(str: &[u8]) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    const SEED: u64 = 0xCBF2_9CE4_8422_2325;

    let mut tail = str;
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
            ::std::mem::transmute::<usize, $ty>(addr + $offset)
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
