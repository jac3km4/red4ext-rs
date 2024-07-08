#![doc = include_str!("../README.md")]
#![allow(clippy::missing_safety_doc)]
use std::ffi::CString;
use std::sync::OnceLock;
use std::{ffi, fmt, mem};

pub use export::{ClassExport, ExportList, ExportNil, Exportable, GlobalExport};
use raw::root::{versioning, RED4ext as red};
use sealed::sealed;
pub use widestring::{widecstr as wcstr, U16CStr};

mod export;
pub mod invocable;
mod raw;
pub mod repr;
pub mod systems;
pub mod types;

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

pub type VoidPtr = *mut std::os::raw::c_void;

pub trait Plugin {
    const NAME: &'static U16CStr;
    const AUTHOR: &'static U16CStr;
    const VERSION: SemVer;
    const SDK: SdkVersion = SdkVersion::LATEST;
    const RUNTIME: RuntimeVersion = RuntimeVersion::RUNTIME_INDEPENDENT;
    const API_VERSION: ApiVersion = ApiVersion::LATEST;

    fn exports() -> impl Exportable {
        ExportNil
    }
    fn on_init(_env: &SdkEnv) {}
}

#[sealed]
pub trait PluginOps: Plugin {
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

#[macro_export]
macro_rules! export_plugin {
    ($trait:ty) => {
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
                <$trait as $crate::PluginOps>::init($crate::SdkEnv::new(handle, sdk));
                $crate::systems::RttiRegistrator::add(Some(on_register), Some(on_post_register));
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

#[cfg(not(feature = "log"))]
pub mod log {
    pub use crate::{debug, error, info, trace, warn};

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

    #[macro_export]
    macro_rules! trace {
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
    pub fn trace(&self, txt: impl fmt::Display) {
        log_internal!(self, Trace, txt);
    }

    #[inline]
    pub fn add_listener(&self, typ: StateType, mut listener: StateListener) -> bool {
        unsafe { ((*self.sdk.gameStates).Add.unwrap())(self.handle, typ as u32, &mut listener.0) }
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
                (*cb_ref).cast::<VoidPtr>(),
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
    pub const RUNTIME_INDEPENDENT: Self = Self(red::FileVer {
        major: versioning::RUNTIME_INDEPENDENT,
        minor: versioning::RUNTIME_INDEPENDENT,
        build: versioning::RUNTIME_INDEPENDENT,
        revision: versioning::RUNTIME_INDEPENDENT,
    });
}

#[derive(Debug)]
pub struct SdkVersion(SemVer);

impl SdkVersion {
    pub const LATEST: Self = Self(SemVer::new(
        versioning::SDK_MAJOR,
        versioning::SDK_MINOR,
        versioning::SDK_PATCH,
    ));
}

#[derive(Debug)]
pub struct ApiVersion(u32);

impl ApiVersion {
    pub const LATEST: Self = Self(versioning::API_VERSION_LATEST);
}

impl From<ApiVersion> for u32 {
    #[inline]
    fn from(api: ApiVersion) -> u32 {
        api.0
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

pub type StateHandler = unsafe extern "C" fn(app: &GameApp);

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

#[derive(Debug)]
#[repr(u32)]
pub enum StateType {
    BaseInitialization = red::EGameStateType::BaseInitialization,
    Initialization = red::EGameStateType::Initialization,
    Running = red::EGameStateType::Running,
    Shutdown = red::EGameStateType::Shutdown,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct PluginInfo(red::PluginInfo);

impl PluginInfo {
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

    #[inline]
    pub fn into_raw(self) -> red::PluginInfo {
        self.0
    }
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
                    ::std::num::NonZero::new($crate::hashes::resolve($crate::hashes::$name)).ok_or(())
                })
                .expect(::std::stringify!(should resolve $name hash))
                .get();
            ::std::mem::transmute::<usize, $ty>(addr + $offset)
        }
        inner()
    }};
}

pub(crate) use fn_from_hash;
