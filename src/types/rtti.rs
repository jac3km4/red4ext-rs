use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::{fmt, iter, mem, ptr, slice};

use super::{
    CName, CNamePool, IAllocator, Native, PoolRef, PoolableOps, RedArray, RedHashMap, RedString,
    ScriptClass, StackArg, StackFrame,
};
use crate::invocable::{Args, InvokeError};
use crate::raw::root::RED4ext as red;
use crate::repr::{FromRepr, NativeRepr};
use crate::systems::RttiSystem;
use crate::VoidPtr;

pub type FunctionHandler<C, R> = extern "C" fn(&C, &mut StackFrame, R, i64);

#[derive(Debug)]
#[repr(transparent)]
pub struct Type(red::CBaseRTTIType);

impl Type {
    #[inline]
    pub(crate) fn as_raw(&self) -> &red::CBaseRTTIType {
        &self.0
    }

    #[inline]
    pub fn name(&self) -> CName {
        // calling Type with unk8 == 0 crashes the game
        if self.0.unk8 == 0 {
            return CName::undefined();
        }
        CName::from_raw(unsafe { (self.vft().tail.CBaseRTTIType_GetName)(&self.0) })
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
    pub fn allocator(&self) -> &IAllocator {
        unsafe { &*((self.vft().tail.CBaseRTTIType_GetAllocator)(&self.0).cast::<IAllocator>()) }
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
    pub fn as_class_mut(&mut self) -> Option<&mut Class> {
        if self.kind().is_class() {
            Some(unsafe { mem::transmute::<&mut red::CBaseRTTIType, &mut Class>(&mut self.0) })
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

    #[allow(clippy::missing_transmute_annotations)]
    pub fn tagged(&self) -> TaggedType<'_> {
        match self.kind() {
            Kind::Name => TaggedType::Name,
            Kind::Fundamental => TaggedType::Fundamental,
            Kind::Class => TaggedType::Class(unsafe { mem::transmute(&self.0) }),
            Kind::Array => TaggedType::Array(unsafe { mem::transmute(&self.0) }),
            Kind::Simple => TaggedType::Simple,
            Kind::Enum => TaggedType::Enum(unsafe { mem::transmute(&self.0) }),
            Kind::StaticArray => TaggedType::StaticArray(unsafe { mem::transmute(&self.0) }),
            Kind::NativeArray => TaggedType::NativeArray(unsafe { mem::transmute(&self.0) }),
            Kind::Pointer => TaggedType::Pointer(unsafe { mem::transmute(&self.0) }),
            Kind::Ref => TaggedType::Ref(unsafe { mem::transmute(&self.0) }),
            Kind::WeakRef => TaggedType::WeakRef(unsafe { mem::transmute(&self.0) }),
            Kind::ResourceRef => TaggedType::ResourceRef(unsafe { mem::transmute(&self.0) }),
            Kind::RaRef => TaggedType::RaRef(unsafe { mem::transmute(&self.0) }),
            Kind::BitField => TaggedType::BitField(unsafe { mem::transmute(&self.0) }),
            Kind::Curve => TaggedType::Curve(unsafe { mem::transmute(&self.0) }),
            Kind::ScriptRef => TaggedType::ScriptRef(unsafe { mem::transmute(&self.0) }),
            Kind::FixedArray => TaggedType::FixedArray(unsafe { mem::transmute(&self.0) }),
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
    Ref = red::ERTTIType::Handle,
    WeakRef = red::ERTTIType::WeakHandle,
    ResourceRef = red::ERTTIType::ResourceReference,
    RaRef = red::ERTTIType::ResourceAsyncReference,
    BitField = red::ERTTIType::BitField,
    Curve = red::ERTTIType::LegacySingleChannelCurve,
    ScriptRef = red::ERTTIType::ScriptReference,
    FixedArray = red::ERTTIType::FixedArray,
}

impl Kind {
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
pub enum TaggedType<'a> {
    Name,
    Fundamental,
    Class(&'a Class),
    Array(&'a ArrayType),
    Simple,
    Enum(&'a Enum),
    StaticArray(&'a StaticArrayType),
    NativeArray(&'a NativeArrayType),
    Pointer(&'a PointerType),
    Ref(&'a RefType),
    WeakRef(&'a WeakRefType),
    ResourceRef(&'a ResourceRefType),
    RaRef(&'a RaRefType),
    BitField(&'a Bitfield),
    Curve(&'a CurveType),
    ScriptRef(&'a ScriptRefType),
    FixedArray(&'a ArrayType),
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Class(red::CClass);

impl Class {
    #[inline]
    fn new_native(name: &CStr, size: u32) -> Self {
        let name = CNamePool::add_cstr(name);
        let mut flags = red::CClass_Flags::default();
        flags.set_isNative(1);
        Self(unsafe { red::CClass::new(name.to_raw(), size, flags) })
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
    pub fn flags(&self) -> ClassFlags {
        ClassFlags(self.0.flags)
    }

    #[inline]
    pub fn set_flags(&mut self, flags: ClassFlags) {
        self.0.flags = flags.0;
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

    #[inline]
    pub fn holder_size(&self) -> u32 {
        self.0.holderSize
    }

    #[inline]
    pub fn alignment(&self) -> u32 {
        self.0.alignment
    }

    #[inline]
    pub fn properties_size(&self) -> u32 {
        if !self.flags().is_native() && self.is_class() {
            self.holder_size()
        } else {
            self.size()
        }
    }

    #[inline]
    pub fn properties(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.props) }
    }

    #[inline]
    pub fn cached_properties(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.unk118) }
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

    pub fn is_class(&self) -> bool {
        // there might be a better way to check this
        iter::once(self)
            .chain(self.base_iter())
            .any(|c| c.name() == CName::new("ISerializable"))
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

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct ClassFlags(red::CClass_Flags);

impl ClassFlags {
    pub fn is_abstract(&self) -> bool {
        self.0.isAbstract() != 0
    }

    pub fn set_is_abstract(&mut self, is_abstract: bool) {
        self.0.set_isAbstract(is_abstract as u32)
    }

    pub fn is_native(&self) -> bool {
        self.0.isNative() != 0
    }

    pub fn set_is_native(&mut self, is_native: bool) {
        self.0.set_isNative(is_native as u32)
    }

    pub fn is_scripted_class(&self) -> bool {
        self.0.isScriptedClass() != 0
    }

    pub fn set_is_scripted_class(&mut self, is_scripted_class: bool) {
        self.0.set_isScriptedClass(is_scripted_class as u32)
    }

    pub fn is_scripted_struct(&self) -> bool {
        self.0.isScriptedStruct() != 0
    }

    pub fn set_is_scripted_struct(&mut self, is_scripted_struct: bool) {
        self.0.set_isScriptedStruct(is_scripted_struct as u32)
    }

    pub fn has_no_default_object_serialization(&self) -> bool {
        self.0.hasNoDefaultObjectSerialization() != 0
    }

    pub fn set_has_no_default_object_serialization(
        &mut self,
        has_no_default_object_serialization: bool,
    ) {
        self.0
            .set_hasNoDefaultObjectSerialization(has_no_default_object_serialization as u32)
    }

    pub fn is_always_transient(&self) -> bool {
        self.0.isAlwaysTransient() != 0
    }

    pub fn set_is_always_transient(&mut self, is_always_transient: bool) {
        self.0.set_isAlwaysTransient(is_always_transient as u32)
    }

    pub fn is_import_only(&self) -> bool {
        self.0.isImportOnly() != 0
    }

    pub fn set_is_import_only(&mut self, is_import_only: bool) {
        self.0.set_isImportOnly(is_import_only as u32)
    }

    pub fn is_private(&self) -> bool {
        self.0.isPrivate() != 0
    }

    pub fn set_is_private(&mut self, is_private: bool) {
        self.0.set_isPrivate(is_private as u32)
    }

    pub fn is_protected(&self) -> bool {
        self.0.isProtected() != 0
    }

    pub fn set_is_protected(&mut self, is_protected: bool) {
        self.0.set_isProtected(is_protected as u32)
    }

    pub fn is_test_only(&self) -> bool {
        self.0.isTestOnly() != 0
    }

    pub fn set_is_test_only(&mut self, is_test_only: bool) {
        self.0.set_isTestOnly(is_test_only as u32)
    }

    pub fn is_savable(&self) -> bool {
        self.0.isSavable() != 0
    }

    pub fn set_is_savable(&mut self, is_savable: bool) {
        self.0.set_isSavable(is_savable as u32)
    }
}

impl fmt::Debug for ClassFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassFlags")
            .field("is_abstract", &self.0.isAbstract())
            .field("is_native", &self.0.isNative())
            .field("is_scripted_class", &self.0.isScriptedClass())
            .field("is_scripted_struct", &self.0.isScriptedStruct())
            .field(
                "has_no_default_object_serialization",
                &self.0.hasNoDefaultObjectSerialization(),
            )
            .field("is_always_transient", &self.0.isAlwaysTransient())
            .field("is_import_only", &self.0.isImportOnly())
            .field("is_private", &self.0.isPrivate())
            .field("is_protected", &self.0.isProtected())
            .field("is_test_only", &self.0.isTestOnly())
            .field("is_savable", &self.0.isSavable())
            .field("b10", &self.0.b10())
            .finish()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NativeClass<T>(Class, PhantomData<*mut T>);

impl<T> NativeClass<T> {
    /// Creates a new native class with the given base type.
    /// Returns a handle to the class, it can only be used to register the class. Any further
    /// use should be done through the RTTI system.
    pub fn new_handle(base: Option<&Class>) -> ClassHandle
    where
        T: Default + Clone + ScriptClass,
    {
        const VFT_SIZE: usize = 30;
        const IS_EQUAL_SLOT: usize = 9;
        const ASSIGN_SLOT: usize = 10;
        const CONSTRUCT_SLOT: usize = 27;
        const DESTRUCT_SLOT: usize = 28;
        const ALLOC_SLOT: usize = 29;

        let cstr = CString::new(T::CLASS_NAME).expect("should create a CString");

        let mut class = Class::new_native(&cstr, mem::size_of::<T>() as u32);
        if let Some(base) = base {
            class.0.parent = base.as_raw() as *const _ as *mut red::CClass;
        }

        let vft = class.as_raw()._base.vtable_ as *mut usize;
        let vft = unsafe { slice::from_raw_parts(vft, VFT_SIZE) };
        let mut vft = vft.to_vec();
        vft[IS_EQUAL_SLOT] = Self::is_equal as _;
        vft[ASSIGN_SLOT] = Self::assign as _;
        vft[CONSTRUCT_SLOT] = Self::construct as _;
        vft[DESTRUCT_SLOT] = Self::destruct as _;
        vft[ALLOC_SLOT] = Self::alloc as _;

        class.0._base.vtable_ = vft.leak().as_ptr() as _;

        // we leak the class and wrap it as pointer, because RTTI expects all references to it
        // to live forever - this prevents accidental misuse
        ClassHandle(NonNull::from(Box::leak(Box::new(class))))
    }

    #[inline]
    pub fn as_class(&self) -> &Class {
        &self.0
    }

    #[inline]
    pub fn as_class_mut(&mut self) -> &mut Class {
        &mut self.0
    }

    fn is_equal(this: VoidPtr, lhs: VoidPtr, rhs: VoidPtr, unk: u32) -> bool {
        unsafe {
            crate::fn_from_hash!(
                TTypedClass_IsEqual,
                unsafe extern "C" fn(VoidPtr, VoidPtr, VoidPtr, u32) -> bool
            )(this, lhs, rhs, unk)
        }
    }

    fn assign(&self, lhs: &mut T, rhs: &T)
    where
        T: Clone,
    {
        lhs.clone_from(rhs)
    }

    fn construct(&self, mem: *mut T)
    where
        T: Default,
    {
        unsafe {
            ptr::write(mem, T::default());
        }
    }

    fn destruct(&self, mem: *mut T) {
        unsafe {
            ptr::drop_in_place(mem);
        }
    }

    fn alloc(&self) -> *mut T {
        let align = self.0.as_type().alignment();
        let size = self.0.as_type().size().next_multiple_of(align);
        let alloc: *mut T = unsafe { self.0.as_type().allocator().alloc_aligned(size, align) };
        alloc
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClassHandle(NonNull<Class>);

impl ClassHandle {
    #[inline]
    pub(crate) fn as_ref(&self) -> &Class {
        unsafe { self.0.as_ref() }
    }

    #[inline]
    pub(crate) fn as_mut(&mut self) -> &mut Class {
        unsafe { self.0.as_mut() }
    }
}

#[derive(Debug)]
pub struct PointerType(red::CRTTIPointerType);

impl PointerType {
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct RefType(red::CRTTIHandleType);

impl RefType {
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct WeakRefType(red::CRTTIWeakHandleType);

impl WeakRefType {
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct ScriptRefType(red::CRTTIScriptReferenceType);

impl ScriptRefType {
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct StaticArrayType(red::CRTTIStaticArrayType);

impl StaticArrayType {
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0._base.innerType.cast::<Type>() }
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size as _
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

#[derive(Debug)]
pub struct NativeArrayType(red::CRTTINativeArrayType);

impl NativeArrayType {
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0._base.innerType.cast::<Type>() }
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size as _
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

#[derive(Debug)]
pub struct ResourceRefType(red::CRTTIResourceReferenceType);

impl ResourceRefType {
    #[inline]
    pub fn resource_type(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct RaRefType(red::CRTTIResourceAsyncReferenceType);

impl RaRefType {
    #[inline]
    pub fn resource_type(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
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

#[derive(Debug)]
pub struct CurveType(red::CRTTILegacySingleChannelCurveType);

impl CurveType {
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0.curveType.cast::<Type>() }
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

#[derive(Debug)]
#[repr(transparent)]
pub struct Function(red::CBaseFunction);

impl Function {
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.fullName)
    }

    #[inline]
    pub fn flags(&self) -> FunctionFlags {
        FunctionFlags(self.0.flags)
    }

    #[inline]
    pub fn set_flags(&mut self, flags: FunctionFlags) {
        self.0.flags = flags.0;
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

    pub fn execute<A, R>(&self, ctx: Option<&IScriptable>, mut args: A) -> Result<R, InvokeError>
    where
        A: Args,
        R: FromRepr,
        R::Repr: Default,
    {
        let mut ret = R::Repr::default();
        let mut out = StackArg::new(&mut ret).ok_or(InvokeError::UnresolvedType(R::Repr::NAME))?;
        let arr = args.to_array()?;

        #[cfg(not(all(debug_assertions, feature = "log")))]
        self.validate_stack(arr.as_ref(), &out)?;

        #[cfg(all(debug_assertions, feature = "log"))]
        if let Err(err) = self.validate_stack(arr.as_ref(), &out) {
            log::error!("Call error: {}", err);
            return Err(err);
        }

        self.execute_internal(ctx, arr.as_ref(), &mut out)?;
        Ok(R::from_repr(ret))
    }

    #[inline(never)]
    fn validate_stack(&self, args: &[StackArg<'_>], ret: &StackArg<'_>) -> Result<(), InvokeError> {
        if self.params().len() != args.len() as u32 {
            return Err(InvokeError::InvalidArgCount {
                function: self.name().as_str(),
                expected: self.params().len(),
            });
        }

        for (index, (param, arg)) in self.params().iter().zip(args.iter()).enumerate() {
            if !arg.type_().is_some_and(|ty| ptr::eq(ty, param.type_())) {
                let expected = param.type_().name().as_str();
                return Err(InvokeError::ArgMismatch {
                    function: self.name().as_str(),
                    expected,
                    index,
                });
            }
        }

        if !ret
            .type_()
            .is_some_and(|ty| ptr::eq(ty, self.return_value().type_()))
        {
            let expected = self.return_value().type_().name().as_str();
            return Err(InvokeError::ReturnMismatch {
                function: self.name().as_str(),
                expected,
            });
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
            Err(InvokeError::ExecutionFailed(self.name().as_str()))
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
        handler: FunctionHandler<IScriptable, R>,
        flags: FunctionFlags,
    ) -> PoolRef<Self> {
        let mut func = GlobalFunction::alloc().expect("should allocate a GlobalFunction");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        Self::ctor(func.as_mut_ptr(), full_name, short_name, handler as _);
        let mut func = unsafe { func.assume_init() };
        func.as_function_mut().set_flags(flags);
        func
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
    pub fn new<C, R>(
        full_name: &CStr,
        short_name: &CStr,
        handler: FunctionHandler<C, R>,
        flags: FunctionFlags,
    ) -> PoolRef<Self>
    where
        C: ScriptClass,
    {
        let mut func = Method::alloc().expect("should allocate a Method");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        let rtti = RttiSystem::get();
        let class = rtti
            .get_class(CName::new(C::CLASS_NAME))
            .expect("should find the class");

        Self::ctor(
            func.as_mut_ptr(),
            class,
            full_name,
            short_name,
            handler as _,
            flags,
        );
        unsafe { func.assume_init() }
    }

    fn ctor(
        ptr: *mut Self,
        class: *const Class,
        full_name: CName,
        short_name: CName,
        handler: VoidPtr,
        flags: FunctionFlags,
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
            ctor(ptr, class, full_name, short_name, handler, flags.0);
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
        handler: FunctionHandler<IScriptable, R>,
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

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct FunctionFlags(red::CBaseFunction_Flags);

impl FunctionFlags {
    pub fn is_native(&self) -> bool {
        self.0.isNative() != 0
    }

    pub fn set_is_native(&mut self, is_native: bool) {
        self.0.set_isNative(is_native as u32)
    }

    pub fn is_static(&self) -> bool {
        self.0.isStatic() != 0
    }

    pub fn set_is_static(&mut self, is_static: bool) {
        self.0.set_isStatic(is_static as u32)
    }

    pub fn is_final(&self) -> bool {
        self.0.isFinal() != 0
    }

    pub fn set_is_final(&mut self, is_final: bool) {
        self.0.set_isFinal(is_final as u32)
    }

    pub fn is_event(&self) -> bool {
        self.0.isEvent() != 0
    }

    pub fn set_is_event(&mut self, is_event: bool) {
        self.0.set_isEvent(is_event as u32)
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
    pub fn flags(&self) -> PropertyFlags {
        PropertyFlags(self.0.flags)
    }

    #[inline]
    pub fn set_flags(&mut self, flags: PropertyFlags) {
        self.0.flags = flags.0;
    }

    #[inline]
    pub fn type_(&self) -> &Type {
        unsafe { &*(self.0.type_ as *const Type) }
    }

    #[inline]
    pub fn value_offset(&self) -> u32 {
        self.0.valueOffset
    }

    #[inline]
    pub unsafe fn value(&self, container: ValueContainer) -> ValuePtr {
        unsafe { ValuePtr(container.0.byte_add(self.0.valueOffset as usize)) }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct PropertyFlags(red::CProperty_Flags);

impl PropertyFlags {
    pub fn is_scripted(&self) -> bool {
        self.0.isScripted() != 0
    }

    pub fn set_is_scripted(&mut self, is_scripted: bool) {
        self.0.set_isScripted(is_scripted as u64)
    }

    pub fn in_value_holder(&self) -> bool {
        self.0.inValueHolder() != 0
    }

    pub fn set_in_value_holder(&mut self, in_value_holder: bool) {
        self.0.set_inValueHolder(in_value_holder as u64)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ArrayType(red::CRTTIBaseArrayType);

impl ArrayType {
    #[inline]
    pub fn inner_type(&self) -> &Type {
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
        unsafe { mem::transmute(&self.0.hashList) }
    }

    #[inline]
    pub fn variant_values(&self) -> &RedArray<i64> {
        unsafe { mem::transmute(&self.0.valueList) }
    }

    #[inline]
    pub fn byte_size(&self) -> u8 {
        self.0.actualSize
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

    pub fn byte_size(&self) -> u8 {
        self.0.actualSize
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
pub struct ISerializable(red::ISerializable);

impl ISerializable {
    #[inline]
    pub fn class(&self) -> &Class {
        unsafe {
            &*(((*self.0.vtable_).ISerializable_GetType)(
                (&self.0) as *const _ as *mut red::ISerializable,
            ) as *const Class)
        }
    }
}

unsafe impl ScriptClass for ISerializable {
    type Kind = Native;

    const CLASS_NAME: &'static str = "ISerializable";
}

#[derive(Debug)]
#[repr(transparent)]
pub struct IScriptable(red::IScriptable);

impl IScriptable {
    #[inline]
    pub fn class(&self) -> &Class {
        self.as_serializable().class()
    }

    #[inline]
    pub fn fields(&self) -> ValueContainer {
        ValueContainer(self.0.valueHolder)
    }

    #[inline]
    pub fn as_serializable(&self) -> &ISerializable {
        unsafe { &*(self as *const _ as *const ISerializable) }
    }

    #[inline]
    pub fn set_native_type(&mut self, class: &Class) {
        self.0.nativeType = class.as_raw() as *const _ as *mut red::CClass;
    }
}

impl Default for IScriptable {
    #[inline]
    fn default() -> Self {
        Self(unsafe { red::IScriptable::new() })
    }
}

impl Clone for IScriptable {
    #[inline]
    fn clone(&self) -> Self {
        unsafe { ptr::read(self) }
    }
}

impl AsRef<IScriptable> for IScriptable {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Drop for IScriptable {
    #[inline]
    fn drop(&mut self) {
        unsafe { red::IScriptable_IScriptable_destructor(&mut self.0) }
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
