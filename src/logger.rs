use std::fmt::Arguments;

use once_cell::sync::OnceCell;

use crate::plugin::{PluginHandle, Sdk};

#[repr(C)]
pub struct SdkLogger {
    trace: fn(PluginHandle, *const u8),
    tracef: fn(PluginHandle, *const u8),

    tracew: fn(PluginHandle, *const u16),
    tracewf: fn(PluginHandle, *const u16),

    debug: fn(PluginHandle, *const u8),
    debugf: fn(PluginHandle, *const u8),

    debugw: fn(PluginHandle, *const u16),
    debugwf: fn(PluginHandle, *const u16),

    info: fn(PluginHandle, *const u8),
    infof: fn(PluginHandle, *const u8),

    infow: fn(PluginHandle, *const u16),
    infowf: fn(PluginHandle, *const u16),

    warn: fn(PluginHandle, *const u8),
    warnf: fn(PluginHandle, *const u8),

    warnw: fn(PluginHandle, *const u16),
    warnwf: fn(PluginHandle, *const u16),

    error: fn(PluginHandle, *const u8),
    errorf: fn(PluginHandle, *const u8),

    errorw: fn(PluginHandle, *const u16),
    errorwf: fn(PluginHandle, *const u16),

    critical: fn(PluginHandle, *const u8),
    criticalf: fn(PluginHandle, *const u8),

    criticalw: fn(PluginHandle, *const u16),
    criticalwf: fn(PluginHandle, *const u16),
}

static INSTANCE: OnceCell<Logger> = OnceCell::new();

pub struct Logger {
    sdk: &'static Sdk,
    handle: PluginHandle,
}

impl Logger {
    pub fn init(sdk: &'static Sdk, handle: PluginHandle) -> Result<(), Self> {
        INSTANCE.set(Self { sdk, handle })
    }

    #[inline]
    pub fn with_logger<F: Fn(&Self)>(func: F) {
        if let Some(logger) = INSTANCE.get() {
            func(logger);
        }
    }

    pub fn error(&self, args: Arguments<'_>) {
        let str = format!("{}\0", args);
        (self.sdk.logger.error)(self.handle, str.as_bytes().as_ptr());
    }

    pub fn warn(&self, args: Arguments<'_>) {
        let str = format!("{}\0", args);
        (self.sdk.logger.warn)(self.handle, str.as_bytes().as_ptr());
    }

    pub fn info(&self, args: Arguments<'_>) {
        let str = format!("{}\0", args);
        (self.sdk.logger.info)(self.handle, str.as_bytes().as_ptr());
    }

    pub fn debug(&self, args: Arguments<'_>) {
        let str = format!("{}\0", args);
        (self.sdk.logger.debug)(self.handle, str.as_bytes().as_ptr());
    }

    pub fn trace(&self, args: Arguments<'_>) {
        let str = format!("{}\0", args);
        (self.sdk.logger.trace)(self.handle, str.as_bytes().as_ptr());
    }
}

#[macro_export]
macro_rules! log {
    ($level:ident, $($args:expr),*) => {
        $crate::logger::Logger::with_logger(|log| log.$level(format_args!($($args),*)))
    };
}

#[macro_export]
macro_rules! error {
    ($($args:expr),*) => { $crate::log!(error, $($args),*) }
}

#[macro_export]
macro_rules! warn {
    ($($args:expr),*) => { $crate::log!(warn, $($args),*) }
}

#[macro_export]
macro_rules! info {
    ($($args:expr),*) => { $crate::log!(info, $($args),*) }
}

#[macro_export]
macro_rules! debug {
    ($($args:expr),*) => { $crate::log!(debug, $($args),*) }
}

#[macro_export]
macro_rules! trace {
    ($($args:expr),*) => { $crate::log!(trace, $($args),*) }
}
