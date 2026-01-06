use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

use crate::{SdkEnv, truncated_cstring};

pub enum SdkEnvWriter<'a> {
    Trace(&'a SdkEnv),
    Debug(&'a SdkEnv),
    Info(&'a SdkEnv),
    Warn(&'a SdkEnv),
    Error(&'a SdkEnv),
}

impl<'a> std::io::Write for SdkEnvWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Ok(x) = std::str::from_utf8(buf) else {
            return Err(std::io::Error::other("invalid utf-8"));
        };
        let x = truncated_cstring(x.to_string());
        unsafe {
            match self {
                Self::Trace(sdk) => ((*sdk.sdk.logger).Trace.unwrap())(sdk.handle, x.as_ptr()),
                Self::Debug(sdk) => ((*sdk.sdk.logger).Debug.unwrap())(sdk.handle, x.as_ptr()),
                Self::Info(sdk) => ((*sdk.sdk.logger).Info.unwrap())(sdk.handle, x.as_ptr()),
                Self::Warn(sdk) => ((*sdk.sdk.logger).Warn.unwrap())(sdk.handle, x.as_ptr()),
                Self::Error(sdk) => ((*sdk.sdk.logger).Error.unwrap())(sdk.handle, x.as_ptr()),
            };
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for &SdkEnv {
    type Writer = SdkEnvWriter<'writer>;

    fn make_writer(&'writer self) -> Self::Writer {
        SdkEnvWriter::Trace(self)
    }

    fn make_writer_for(&'writer self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        match *meta.level() {
            Level::TRACE => SdkEnvWriter::Trace(self),
            Level::DEBUG => SdkEnvWriter::Debug(self),
            Level::INFO => SdkEnvWriter::Info(self),
            Level::WARN => SdkEnvWriter::Warn(self),
            Level::ERROR => SdkEnvWriter::Error(self),
        }
    }
}
