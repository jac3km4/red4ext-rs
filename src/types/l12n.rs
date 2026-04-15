use crate::raw::root::RED4ext as red;

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

impl std::fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            unsafe { std::ffi::CStr::from_ptr(self.0.unk08.c_str()) }
                .to_str()
                .unwrap_or_default()
        )
    }
}
