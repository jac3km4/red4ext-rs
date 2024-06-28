use std::ffi::CStr;
use std::{iter, mem, ptr};

use super::{
    CName, CNamePool, IAllocator, Native, PoolRef, PoolableOps, RedArray, RedHashMap, RedString,
    ScriptClass, StackArg, StackFrame,
};
use crate::invocable::{Args, InvokeError};
use crate::raw::root::RED4ext as red;
use crate::repr::{FromRepr, NativeRepr};
use crate::VoidPtr;

pub type FunctionHandler<R> = extern "C" fn(Option<&IScriptable>, &mut StackFrame, R, i64);

#[derive(Debug)]
#[repr(transparent)]
pub struct Type(red::CBaseRTTIType);

impl Type {
    #[inline]
    pub fn name(&self) -> CName {
        // calling Type with unk8 == 0 crashes the game
        if self.0.unk8 == 0 {
            return CName::undefined();
        }
        CName::from_raw(unsafe { (self.vft().tail.CBaseRTTIType_GetName)(&self.0) })
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> &red::CBaseRTTIType {
        &self.0
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

    pub unsafe fn to_string(&self, value: ValuePtr) -> RedString {
        let mut str = RedString::new();
        unsafe {
            (self.vft().tail.CBaseRTTIType_ToString)(
                &self.0,
                value.0,
                &mut str as *mut _ as *mut red::CString,
            )
        };
        str
    }

    #[inline]
    fn vft(&self) -> &TypeVft {
        unsafe { &*(self.0.vtable_.cast::<TypeVft>()) }
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
#[repr(transparent)]
pub struct Class(red::CClass);

impl Class {
    #[inline]
    pub fn new(name: &CStr, size: u32) -> Self {
        let name = CNamePool::add_cstr(name);
        Self(unsafe { red::CClass::new(name.to_raw(), size, Default::default()) })
    }

    #[inline]
    pub(super) fn as_raw(&self) -> &red::CClass {
        &self.0
    }

    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    #[inline]
    pub fn properties(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.props) }
    }

    #[inline]
    pub fn methods(&self) -> &RedArray<&Method> {
        unsafe { mem::transmute(&self.0.funcs) }
    }

    #[inline]
    pub fn method_map(&self) -> &RedHashMap<CName, &Method> {
        unsafe { mem::transmute(&self.0.funcsByName) }
    }

    pub fn get_method(&self, name: CName) -> Option<&Method> {
        iter::once(self)
            .chain(self.base_iter())
            .find_map(|class| class.method_map().get(&name).copied())
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

    #[inline]
    pub fn instantiate(&self) -> ValueContainer {
        ValueContainer(unsafe { self.0.CreateInstance(true) })
    }

    #[inline]
    pub fn add_method(&mut self, func: PoolRef<Method>) {
        self.methods_mut().push(&func);
        // RTTI takes ownership of it from now on
        mem::forget(func);
    }

    #[inline]
    pub fn add_static_method(&mut self, func: PoolRef<StaticMethod>) {
        self.static_methods_mut().push(&func);
        // RTTI takes ownership of it from now on
        mem::forget(func);
    }

    #[inline]
    pub fn add_property(&mut self, prop: PoolRef<Property>) {
        self.properties_mut().push(&prop);
        // RTTI takes ownership of it from now on
        mem::forget(prop);
    }

    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }

    #[inline]
    fn methods_mut(&mut self) -> &mut RedArray<&Method> {
        unsafe { mem::transmute(&mut self.0.funcs) }
    }

    #[inline]
    fn static_methods_mut(&mut self) -> &mut RedArray<&StaticMethod> {
        unsafe { mem::transmute(&mut self.0.staticFuncs) }
    }

    #[inline]
    fn properties_mut(&mut self) -> &mut RedArray<&Property> {
        unsafe { mem::transmute(&mut self.0.props) }
    }
}

impl Drop for Class {
    #[inline]
    fn drop(&mut self) {
        let t = self.as_type_mut();
        unsafe { (t.vft().destroy)(t) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Function(red::CBaseFunction);

impl Function {
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.fullName)
    }

    #[inline]
    pub fn parent(&self) -> Option<&Class> {
        unsafe { (self.vft().get_parent)(self).as_ref() }
    }

    #[inline]
    pub fn locals(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.localVars) }
    }

    #[inline]
    pub fn params(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.params) }
    }

    #[inline]
    pub fn return_value(&self) -> &Property {
        unsafe { &*(self.0.returnType.cast::<Property>()) }
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        self.0.flags.isStatic() != 0
    }

    #[inline]
    pub fn add_param(&mut self, typ: CName, name: &CStr, is_out: bool, is_optional: bool) -> bool {
        unsafe {
            self.0
                .AddParam(typ.to_raw(), name.as_ptr(), is_out, is_optional)
        }
    }

    #[inline]
    pub fn set_return_type(&mut self, typ: CName) {
        unsafe { self.0.SetReturnType(typ.to_raw()) };
    }

    #[inline]
    pub fn set_is_native(&mut self, is_native: bool) {
        self.0.flags.set_isNative(is_native as u32)
    }

    #[inline]
    pub fn set_is_final(&mut self, is_final: bool) {
        self.0.flags.set_isFinal(is_final as u32)
    }

    #[inline]
    pub fn set_is_static(&mut self, is_static: bool) {
        self.0.flags.set_isStatic(is_static as u32)
    }

    pub fn execute<A, R>(&self, ctx: Option<&IScriptable>, mut args: A) -> Result<R, InvokeError>
    where
        A: Args,
        R: FromRepr,
        R::Repr: Default,
    {
        let mut ret = R::Repr::default();
        let mut out =
            StackArg::new(&mut ret).ok_or(InvokeError::UnresolvedType(R::Repr::NATIVE_NAME))?;
        let arr = args.to_array()?;
        self.validate_stack(arr.as_ref(), &out)?;
        self.execute_internal(ctx, arr.as_ref(), &mut out)?;
        Ok(R::from_repr(ret))
    }

    #[inline(never)]
    fn validate_stack(&self, args: &[StackArg<'_>], ret: &StackArg<'_>) -> Result<(), InvokeError> {
        if self.params().len() != args.len() as u32 {
            return Err(InvokeError::InvalidArgCount(self.params().len()));
        }

        for (index, (param, arg)) in self.params().iter().zip(args.iter()).enumerate() {
            if !arg.type_().is_some_and(|ty| ptr::eq(ty, param.type_())) {
                let expected = param.type_().name().as_str();
                return Err(InvokeError::ArgMismatch { expected, index });
            }
        }

        if !ret
            .type_()
            .is_some_and(|ty| ptr::eq(ty, self.return_value().type_()))
        {
            let expected = self.return_value().type_().name().as_str();
            return Err(InvokeError::ReturnMismatch { expected });
        }

        Ok(())
    }

    fn execute_internal(
        &self,
        ctx: Option<&IScriptable>,
        args: &[StackArg<'_>],
        ret: &mut StackArg<'_>,
    ) -> Result<(), InvokeError> {
        let success = unsafe {
            let mut stack = red::CStack::new(
                mem::transmute::<Option<&IScriptable>, VoidPtr>(ctx),
                mem::transmute::<*const StackArg<'_>, *mut red::CStackType>(args.as_ptr()),
                args.len() as u32,
                ret.as_raw_mut(),
            );
            red::CBaseFunction_Execute(&self.0 as *const _ as *mut red::CBaseFunction, &mut stack)
        };
        if success {
            Ok(())
        } else {
            Err(InvokeError::ExecutionFailed)
        }
    }

    #[inline]
    fn vft(&self) -> &FunctionVft {
        unsafe { &*(self.0._base.vtable_.cast::<FunctionVft>()) }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct GlobalFunction(red::CGlobalFunction);

impl GlobalFunction {
    pub fn new<R>(
        full_name: &CStr,
        short_name: &CStr,
        handler: FunctionHandler<R>,
    ) -> PoolRef<Self> {
        let mut func = GlobalFunction::alloc().expect("should allocate a GlobalFunction");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        Self::ctor(func.as_mut_ptr(), full_name, short_name, handler as _);
        unsafe { func.assume_init() }
    }

    fn ctor(ptr: *mut Self, full_name: CName, short_name: CName, handler: VoidPtr) {
        unsafe {
            let ctor = crate::fn_from_hash!(
                CGlobalFunction_ctor,
                unsafe extern "C" fn(*mut GlobalFunction, CName, CName, VoidPtr)
            );
            ctor(ptr, full_name, short_name, handler);
        };
    }

    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    #[inline]
    pub fn as_function_mut(&mut self) -> &mut Function {
        unsafe { &mut *(self as *mut _ as *mut Function) }
    }
}

impl Drop for GlobalFunction {
    #[inline]
    fn drop(&mut self) {
        let f = self.as_function_mut();
        unsafe { (f.vft().destruct)(f) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Method(red::CClassFunction);

impl Method {
    pub fn new<R>(
        full_name: &CStr,
        short_name: &CStr,
        class: &Class,
        handler: FunctionHandler<R>,
    ) -> PoolRef<Self> {
        let mut func = Method::alloc().expect("should allocate a Method");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        Self::ctor(
            func.as_mut_ptr(),
            class,
            full_name,
            short_name,
            handler as _,
        );
        unsafe { func.assume_init() }
    }

    fn ctor(
        ptr: *mut Self,
        class: *const Class,
        full_name: CName,
        short_name: CName,
        handler: VoidPtr,
    ) {
        unsafe {
            let ctor = crate::fn_from_hash!(
                CClassFunction_ctor,
                unsafe extern "C" fn(
                    *mut Method,
                    *const Class,
                    CName,
                    CName,
                    VoidPtr,
                    red::CBaseFunction_Flags,
                )
            );
            ctor(
                ptr,
                class,
                full_name,
                short_name,
                handler,
                Default::default(),
            );
        };
    }

    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    #[inline]
    pub fn as_function_mut(&mut self) -> &mut Function {
        unsafe { &mut *(self as *mut _ as *mut Function) }
    }
}

impl Drop for Method {
    #[inline]
    fn drop(&mut self) {
        let f = self.as_function_mut();
        unsafe { (f.vft().destruct)(f) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct StaticMethod(red::CClassStaticFunction);

impl StaticMethod {
    pub fn new<R>(
        full_name: &CStr,
        short_name: &CStr,
        class: &Class,
        handler: FunctionHandler<R>,
    ) -> PoolRef<Self> {
        let mut func = StaticMethod::alloc().expect("should allocate a StaticMethod");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        Self::ctor(
            func.as_mut_ptr(),
            class,
            full_name,
            short_name,
            handler as _,
        );
        unsafe { func.assume_init() }
    }

    fn ctor(
        ptr: *mut Self,
        class: *const Class,
        full_name: CName,
        short_name: CName,
        handler: VoidPtr,
    ) {
        unsafe {
            let ctor = crate::fn_from_hash!(
                CClassStaticFunction_ctor,
                unsafe extern "C" fn(
                    *mut StaticMethod,
                    *const Class,
                    CName,
                    CName,
                    VoidPtr,
                    red::CBaseFunction_Flags,
                )
            );
            ctor(
                ptr,
                class,
                full_name,
                short_name,
                handler,
                Default::default(),
            );
        };
    }

    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    #[inline]
    pub fn as_function_mut(&mut self) -> &mut Function {
        unsafe { &mut *(self as *mut _ as *mut Function) }
    }
}

impl Drop for StaticMethod {
    #[inline]
    fn drop(&mut self) {
        let f = self.as_function_mut();
        unsafe { (f.vft().destruct)(f) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Property(red::CProperty);

impl Property {
    pub fn new(
        name: &CStr,
        group: &CStr,
        type_: &Type,
        parent: &Class,
        offset: u32,
    ) -> PoolRef<Self> {
        let mut prop = Property::alloc().expect("should allocate a Property");
        let name = CNamePool::add_cstr(name);
        let group = CNamePool::add_cstr(group);

        let ptr = prop.as_mut_ptr();
        unsafe {
            (*ptr).0.name = name.to_raw();
            (*ptr).0.group = group.to_raw();
            (*ptr).0.type_ = type_.as_raw() as *const _ as *mut red::CBaseRTTIType;
            (*ptr).0.parent = parent.as_raw() as *const _ as *mut red::CClass;
            (*ptr).0.valueOffset = offset;
            prop.assume_init()
        }
    }

    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
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
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }

    #[inline]
    fn vft(&self) -> &ArrayTypeVft {
        unsafe { &*(self.0._base.vtable_ as *const ArrayTypeVft) }
    }
}

impl Drop for ArrayType {
    #[inline]
    fn drop(&mut self) {
        let t = self.as_type_mut();
        unsafe { (t.vft().destroy)(t) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Enum(red::CEnum);

impl Enum {
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    #[inline]
    pub fn variant_names(&self) -> &RedArray<CName> {
        unsafe { mem::transmute(&self.0.aliasList) }
    }

    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

impl Drop for Enum {
    #[inline]
    fn drop(&mut self) {
        let t = self.as_type_mut();
        unsafe { (t.vft().destroy)(t) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Bitfield(red::CBitfield);

impl Bitfield {
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    pub fn fields(&self) -> &[CName; 64] {
        unsafe { mem::transmute(&self.0.bitNames) }
    }

    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

impl Drop for Bitfield {
    #[inline]
    fn drop(&mut self) {
        let t = self.as_type_mut();
        unsafe { (t.vft().destroy)(t) };
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct IScriptable(red::IScriptable);

impl IScriptable {
    pub fn class(&self) -> &'static Class {
        unsafe {
            &*(((*self.0._base.vtable_).ISerializable_GetType)(
                (&self.0._base) as *const _ as *mut red::ISerializable,
            ) as *const Class)
        }
    }

    #[inline]
    pub fn fields(&self) -> ValueContainer {
        ValueContainer(self.0.valueHolder)
    }
}

impl AsRef<IScriptable> for IScriptable {
    #[inline]
    fn as_ref(&self) -> &IScriptable {
        self
    }
}

impl AsMut<IScriptable> for IScriptable {
    #[inline]
    fn as_mut(&mut self) -> &mut IScriptable {
        self
    }
}

unsafe impl ScriptClass for IScriptable {
    type Kind = Native;

    const CLASS_NAME: &'static str = "IScriptable";
}

#[derive(Debug, Clone, Copy)]
pub struct ValueContainer(VoidPtr);

impl ValueContainer {
    #[inline]
    pub(super) fn new(ptr: VoidPtr) -> Self {
        Self(ptr)
    }

    #[inline]
    pub(super) fn as_ptr(&self) -> VoidPtr {
        self.0
    }
}

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

    #[inline]
    pub unsafe fn to_container(&self) -> ValueContainer {
        ValueContainer(self.0)
    }
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
    get_allocator: unsafe extern "fastcall" fn(this: &Function) -> *mut IAllocator,
    destruct: unsafe extern "fastcall" fn(this: &mut Function),
    get_parent: unsafe extern "fastcall" fn(this: &Function) -> *mut Class,
}
