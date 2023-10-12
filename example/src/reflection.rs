use red4ext_rs::prelude::{redscript_import, ClassType, NativeRepr};
use red4ext_rs::types::{CName, IScriptable, MaybeUninitRef, Ref, Variant};

#[derive(Debug)]
pub struct Reflection;

unsafe impl NativeRepr for Reflection {
    const NAME: &'static str = "Reflection";
}

#[redscript_import]
impl Reflection {
    /// `public static native func GetClass(name: CName) -> ref<ReflectionClass>`
    #[redscript(native, name = "GetClass")]
    pub fn get_class(name: CName) -> MaybeUninitRef<ReflectionClass>;
}

#[derive(Debug)]
pub struct ReflectionClass;

impl ClassType for ReflectionClass {
    type BaseClass = ReflectionType;

    const NAME: &'static str = "ReflectionClass";
}

#[redscript_import]
impl ReflectionClass {
    /// `public native func GetProperty(name: CName) -> ref<ReflectionProp>`
    #[redscript(native, name = "GetProperty")]
    pub fn get_property(self: &Ref<Self>, name: CName) -> MaybeUninitRef<ReflectionProp>;
}

#[derive(Debug)]
pub struct ReflectionProp;

impl ClassType for ReflectionProp {
    type BaseClass = IScriptable;

    const NAME: &'static str = "ReflectionProp";
}

#[redscript_import]
impl ReflectionProp {
    /// `public native func GetValue(owner: Variant) -> Variant`
    #[redscript(native)]
    pub fn get_value(self: &Ref<Self>, owner: Variant) -> Variant;
}

#[derive(Debug)]
pub struct ReflectionType;

impl ClassType for ReflectionType {
    type BaseClass = IScriptable;

    const NAME: &'static str = "ReflectionType";
}
