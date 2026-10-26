use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::{fmt, iter, mem, ptr, slice};

use super::{
    CName, CNamePool, IAllocator, PoolRef, PoolableOps, RedArray, RedHashMap, RedString, StackArg,
    StackFrame, WeakRef,
};
use crate::invocable::{Args, InvokeError};
use crate::raw::root::RED4ext as red;
use crate::repr::{FromRepr, NativeRepr};
use crate::types::PtrEq;
use crate::{ScriptClass, VoidPtr, check_invariant, class_kind};

/// Native function callback receiving a context, script stack, return storage, and engine argument.
///
/// The callback must obey the engine calling convention and match its registered signature.
///
/// SDK: [functions](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
pub type FunctionHandler<C, R> = unsafe extern "C" fn(*mut C, &mut StackFrame, R, i64);

/// Runtime type metadata describing a value's name, layout, and type category.
///
/// Metadata borrowed from RTTI remains engine-owned. Engine RTTI must be initialized
/// before querying types or operating on their values.
///
/// SDK: [CBaseRTTIType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Type(red::rtti::IType);

impl Type {
    #[inline]
    pub(crate) fn as_raw(&self) -> &red::rtti::IType {
        &self.0
    }

    #[inline]
    pub(crate) unsafe fn from_raw<'a>(raw: *const red::rtti::IType) -> Option<&'a Self> {
        unsafe { raw.cast::<Self>().as_ref() }
    }

    /// Returns the RTTI name, or an undefined name when name metadata is absent.
    #[inline]
    pub fn name(&self) -> CName {
        // calling Type with unk8 == 0 crashes the game
        if self.0.unk8.is_null() {
            return CName::undefined();
        }
        CName::from_raw(unsafe { (self.vft().tail.IType_GetName)(&self.0) })
    }

    /// Returns the size of a value in bytes.
    #[inline]
    pub fn size(&self) -> u32 {
        unsafe { (self.vft().tail.IType_GetSize)(&self.0) }
    }

    /// Returns the required value alignment in bytes.
    #[inline]
    pub fn alignment(&self) -> u32 {
        unsafe { (self.vft().tail.IType_GetAlignment)(&self.0) }
    }

    /// Returns the category used to interpret this metadata.
    #[inline]
    pub fn kind(&self) -> TypeKind {
        unsafe { mem::transmute((self.vft().tail.IType_GetType)(&self.0)) }
    }

    /// Returns the engine allocator associated with this type.
    #[inline]
    pub fn allocator(&self) -> &IAllocator {
        unsafe { &*((self.vft().tail.IType_GetAllocator)(&self.0).cast::<IAllocator>()) }
    }

    /// Returns class or struct metadata, or `None` for other type categories.
    #[inline]
    pub fn as_class(&self) -> Option<&Class> {
        if self.kind().is_class() {
            Some(unsafe { mem::transmute::<&red::rtti::IType, &Class>(&self.0) })
        } else {
            None
        }
    }

    /// Returns mutable class or struct metadata, or `None` for other categories.
    #[inline]
    pub fn as_class_mut(&mut self) -> Option<&mut Class> {
        if self.kind().is_class() {
            Some(unsafe { mem::transmute::<&mut red::rtti::IType, &mut Class>(&mut self.0) })
        } else {
            None
        }
    }

    /// Returns common array metadata for any of the four array categories.
    #[inline]
    pub fn as_array(&self) -> Option<&ArrayType> {
        if self.kind().is_array() {
            Some(unsafe { mem::transmute::<&red::rtti::IType, &ArrayType>(&self.0) })
        } else {
            None
        }
    }

    /// Selects a borrowed, type-specific metadata view using this type's category.
    #[allow(clippy::missing_transmute_annotations)]
    pub fn tagged(&self) -> TaggedType<'_> {
        match self.kind() {
            TypeKind::Name => TaggedType::Name,
            TypeKind::Fundamental => TaggedType::Fundamental,
            TypeKind::Class => TaggedType::Class(unsafe { mem::transmute(&self.0) }),
            TypeKind::Array => TaggedType::Array(unsafe { mem::transmute(&self.0) }),
            TypeKind::Simple => TaggedType::Simple,
            TypeKind::Enum => TaggedType::Enum(unsafe { mem::transmute(&self.0) }),
            TypeKind::StaticArray => TaggedType::StaticArray(unsafe { mem::transmute(&self.0) }),
            TypeKind::NativeArray => TaggedType::NativeArray(unsafe { mem::transmute(&self.0) }),
            TypeKind::Pointer => TaggedType::Pointer(unsafe { mem::transmute(&self.0) }),
            TypeKind::Ref => TaggedType::Ref(unsafe { mem::transmute(&self.0) }),
            TypeKind::WeakRef => TaggedType::WeakRef(unsafe { mem::transmute(&self.0) }),
            TypeKind::ResourceRef => TaggedType::ResourceRef(unsafe { mem::transmute(&self.0) }),
            TypeKind::RaRef => TaggedType::RaRef(unsafe { mem::transmute(&self.0) }),
            TypeKind::BitField => TaggedType::BitField(unsafe { mem::transmute(&self.0) }),
            TypeKind::Curve => TaggedType::Curve(unsafe { mem::transmute(&self.0) }),
            TypeKind::ScriptRef => TaggedType::ScriptRef(unsafe { mem::transmute(&self.0) }),
            TypeKind::FixedArray => TaggedType::FixedArray(unsafe { mem::transmute(&self.0) }),
        }
    }

    /// Formats a value using the engine's representation for this type.
    ///
    /// # Safety
    /// `value` must point to a live, initialized value of this exact RTTI type,
    /// properly aligned and readable for the duration of the call. The value must
    /// not be concurrently mutated or destroyed.
    pub unsafe fn to_string(&self, value: ValuePtr) -> RedString {
        let mut str = RedString::new();
        unsafe {
            (self.vft().tail.IType_ToString)(
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

/// Engine RTTI category used to select a type-specific metadata view.
///
/// SDK: [ERTTIType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeKind {
    /// An interned name (`CName`).
    Name = red::rtti::ERTTIType::Name,
    /// A primitive value, such as `Int32` or `Bool`.
    Fundamental = red::rtti::ERTTIType::Fundamental,
    /// A reflected class or struct.
    Class = red::rtti::ERTTIType::Class,
    /// A dynamic array.
    Array = red::rtti::ERTTIType::Array,
    /// An engine-defined simple value type.
    Simple = red::rtti::ERTTIType::Simple,
    /// A named enumeration with integer values.
    Enum = red::rtti::ERTTIType::Enum,
    /// A static array with a size recorded in its metadata.
    StaticArray = red::rtti::ERTTIType::StaticArray,
    /// A native array with a size recorded in its metadata.
    NativeArray = red::rtti::ERTTIType::NativeArray,
    /// A raw pointer to another reflected type.
    Pointer = red::rtti::ERTTIType::Pointer,
    /// A strong, reference-counted object handle.
    Ref = red::rtti::ERTTIType::Handle,
    /// A weak object handle.
    WeakRef = red::rtti::ERTTIType::WeakHandle,
    /// A resource reference.
    ResourceRef = red::rtti::ERTTIType::ResourceReference,
    /// An asynchronous resource reference.
    RaRef = red::rtti::ERTTIType::ResourceAsyncReference,
    /// A value with named bits.
    BitField = red::rtti::ERTTIType::BitField,
    /// A legacy single-channel curve.
    Curve = red::rtti::ERTTIType::LegacySingleChannelCurve,
    /// A script reference to a value of another reflected type.
    ScriptRef = red::rtti::ERTTIType::ScriptReference,
    /// An engine fixed-size array.
    FixedArray = red::rtti::ERTTIType::FixedArray,
}

impl TypeKind {
    /// Returns whether this is the class/struct metadata category.
    #[inline]
    pub fn is_class(self) -> bool {
        self == Self::Class
    }

    /// Returns whether this is a dynamic, static, native, or fixed array category.
    #[inline]
    pub fn is_array(self) -> bool {
        matches!(
            self,
            Self::Array | Self::StaticArray | Self::NativeArray | Self::FixedArray
        )
    }
}

/// Borrowed RTTI metadata dispatched by [`TypeKind`].
#[derive(Debug)]
pub enum TaggedType<'a> {
    /// A name type.
    Name,
    /// A fundamental type (e.g. `Int32`).
    Fundamental,
    /// A class type.
    Class(&'a Class),
    /// An array type.
    Array(&'a ArrayType),
    /// A simple type.
    Simple,
    /// An enum type.
    Enum(&'a Enum),
    /// A static array type with a fixed size.
    StaticArray(&'a StaticArrayType),
    /// A native array type with a fixed size.
    NativeArray(&'a NativeArrayType),
    /// A pointer type.
    Pointer(&'a PointerType),
    /// A reference type.
    Ref(&'a RefType),
    /// A weak reference type.
    WeakRef(&'a WeakRefType),
    /// A resource reference type.
    ResourceRef(&'a ResourceRefType),
    /// An asynchronous resource reference type.
    RaRef(&'a RaRefType),
    /// A bitfield type.
    BitField(&'a Bitfield),
    /// A curve type.
    Curve(&'a CurveType),
    /// A script reference type.
    ScriptRef(&'a ScriptRefType),
    /// A fixed size array type.
    FixedArray(&'a ArrayType),
}

/// Reflected class or struct metadata, including inheritance, properties, and methods.
///
/// SDK: [CClass](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
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

    /// Returns the reflected class or struct name.
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    /// Returns a copy of the flags; use `set_flags` to apply changes.
    #[inline]
    pub fn flags(&self) -> ClassFlags {
        ClassFlags(self.0.flags)
    }

    /// Replaces the stored flags with `flags`.
    #[inline]
    pub fn set_flags(&mut self, flags: ClassFlags) {
        self.0.flags = flags.0;
    }

    /// Returns the native instance size in bytes, excluding a separate value holder.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

    /// Returns the size of the separate script value holder in bytes.
    #[inline]
    pub fn holder_size(&self) -> u32 {
        self.0.holderSize
    }

    /// Returns the required instance alignment in bytes.
    #[inline]
    pub fn alignment(&self) -> u32 {
        self.0.alignment
    }

    /// Returns holder size for non-native serializable classes, otherwise instance size.
    #[inline]
    pub fn properties_size(&self) -> u32 {
        if !self.flags().is_native() && self.is_class() {
            self.holder_size()
        } else {
            self.size()
        }
    }

    /// Returns properties declared on this class, excluding inherited properties.
    #[inline]
    pub fn properties(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.props) }
    }

    /// Returns the engine-maintained property cache as currently populated.
    #[inline]
    pub fn cached_properties(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.unk118) }
    }

    /// Returns instance methods declared on this class.
    #[inline]
    pub fn methods(&self) -> &RedArray<&Method> {
        unsafe { mem::transmute(&self.0.funcs) }
    }

    /// Returns this class's engine-maintained instance-method index by full name.
    #[inline]
    pub fn method_map(&self) -> &RedHashMap<CName, &Method> {
        unsafe { mem::transmute(&self.0.funcsByName) }
    }

    /// Returns static methods declared on this class.
    #[inline]
    pub fn static_methods(&self) -> &RedArray<&StaticMethod> {
        unsafe { mem::transmute(&self.0.staticFuncs) }
    }

    /// Resolves a method by name.
    /// Returns the method wrapped in [`Ok`] if it exists in this class or any of its base classes.
    /// If the method is not found, this function returns an iterator over all methods with a
    /// matching short name wrapped in [`Err`].
    pub fn get_method(&self, name: CName) -> Result<&Method, impl Iterator<Item = &Method>> {
        self.base_iter_with_self()
            .find_map(|class| class.method_map().get(&name).copied())
            .ok_or_else(|| {
                self.base_iter_with_self()
                    .flat_map(Class::methods)
                    .filter(move |m| m.as_function().short_name() == name)
                    .copied()
            })
    }

    /// Returns the immediate base class, if one exists.
    #[inline]
    pub fn base(&self) -> Option<&Class> {
        unsafe { (self.0.parent as *const Class).as_ref() }
    }

    /// Iterates from the immediate base to the root, excluding this class.
    #[inline]
    pub fn base_iter(&self) -> impl Iterator<Item = &Class> {
        iter::successors(self.base(), |class| class.base())
    }

    /// Iterates properties of this class, then each base class up to the root.
    /// Overridden properties are not deduplicated.
    pub fn all_properties(&self) -> impl Iterator<Item = &Property> {
        self.base_iter_with_self()
            .flat_map(Class::properties)
            .copied()
    }

    /// Returns whether this type is or derives from `ISerializable`, rather than a struct.
    pub fn is_class(&self) -> bool {
        // there might be a better way to check this
        self.base_iter_with_self()
            .any(|c| c.name() == CName::new("ISerializable"))
    }

    /// Creates an instance through engine RTTI and returns its storage address.
    ///
    /// Requires initialized engine systems. The returned container does not manage
    /// ownership; the instance must be released through the appropriate engine API.
    #[inline]
    pub fn instantiate(&self) -> ValueContainer {
        ValueContainer(unsafe { self.0.CreateInstance(true) })
    }

    /// Appends an instance method, transferring its ownership to RTTI.
    /// This does not update the method-name index.
    #[inline]
    pub fn add_method(&mut self, func: PoolRef<Method>) {
        self.methods_mut().push(&func);
        // RTTI takes ownership of it from now on
        mem::forget(func);
    }

    /// Appends a static method, transferring its ownership to RTTI.
    #[inline]
    pub fn add_static_method(&mut self, func: PoolRef<StaticMethod>) {
        self.static_methods_mut().push(&func);
        // RTTI takes ownership of it from now on
        mem::forget(func);
    }

    /// Appends a property, transferring its ownership to RTTI.
    /// This does not resize instances or rebuild property caches.
    #[inline]
    pub fn add_property(&mut self, prop: PoolRef<Property>) {
        self.properties_mut().push(&prop);
        // RTTI takes ownership of it from now on
        mem::forget(prop);
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }

    #[inline]
    pub(crate) fn base_iter_with_self(&self) -> impl Iterator<Item = &Class> {
        iter::once(self).chain(self.base_iter())
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

/// Class classification, visibility, and serialization flags.
///
/// Getters on [`Class`] return a copy; apply changes with [`Class::set_flags`].
/// Setting a flag changes metadata only, not the class's layout or implementation.
///
/// SDK: [CClass flags](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct ClassFlags(red::CClass_Flags);

impl ClassFlags {
    /// Returns whether the class is marked abstract.
    pub fn is_abstract(&self) -> bool {
        self.0.isAbstract() != 0
    }

    /// Sets whether the class is marked abstract.
    pub fn set_is_abstract(&mut self, is_abstract: bool) {
        self.0.set_isAbstract(is_abstract as u32)
    }

    /// Returns whether the class is implemented natively rather than in script.
    pub fn is_native(&self) -> bool {
        self.0.isNative() != 0
    }

    /// Sets the native flag without changing the class layout or implementation.
    pub fn set_is_native(&mut self, is_native: bool) {
        self.0.set_isNative(is_native as u32)
    }

    /// Returns whether this is a script-defined class.
    pub fn is_scripted_class(&self) -> bool {
        self.0.isScriptedClass() != 0
    }

    /// Sets the script-defined class flag.
    pub fn set_is_scripted_class(&mut self, is_scripted_class: bool) {
        self.0.set_isScriptedClass(is_scripted_class as u32)
    }

    /// Returns whether this is a script-defined struct.
    pub fn is_scripted_struct(&self) -> bool {
        self.0.isScriptedStruct() != 0
    }

    /// Sets the script-defined struct flag.
    pub fn set_is_scripted_struct(&mut self, is_scripted_struct: bool) {
        self.0.set_isScriptedStruct(is_scripted_struct as u32)
    }

    /// Returns whether default-object serialization is disabled.
    pub fn has_no_default_object_serialization(&self) -> bool {
        self.0.hasNoDefaultObjectSerialization() != 0
    }

    /// Sets whether default-object serialization is disabled.
    pub fn set_has_no_default_object_serialization(
        &mut self,
        has_no_default_object_serialization: bool,
    ) {
        self.0
            .set_hasNoDefaultObjectSerialization(has_no_default_object_serialization as u32)
    }

    /// Returns whether instances are marked always transient.
    pub fn is_always_transient(&self) -> bool {
        self.0.isAlwaysTransient() != 0
    }

    /// Sets whether instances are marked always transient.
    pub fn set_is_always_transient(&mut self, is_always_transient: bool) {
        self.0.set_isAlwaysTransient(is_always_transient as u32)
    }

    /// Returns whether the class is marked import-only.
    pub fn is_import_only(&self) -> bool {
        self.0.isImportOnly() != 0
    }

    /// Sets the import-only class flag.
    pub fn set_is_import_only(&mut self, is_import_only: bool) {
        self.0.set_isImportOnly(is_import_only as u32)
    }

    /// Returns whether the class has private visibility.
    pub fn is_private(&self) -> bool {
        self.0.isPrivate() != 0
    }

    /// Sets the private visibility flag without changing other visibility flags.
    pub fn set_is_private(&mut self, is_private: bool) {
        self.0.set_isPrivate(is_private as u32)
    }

    /// Returns whether the class has protected visibility.
    pub fn is_protected(&self) -> bool {
        self.0.isProtected() != 0
    }

    /// Sets the protected visibility flag without changing other visibility flags.
    pub fn set_is_protected(&mut self, is_protected: bool) {
        self.0.set_isProtected(is_protected as u32)
    }

    /// Returns whether the class is marked for test-only use.
    pub fn is_test_only(&self) -> bool {
        self.0.isTestOnly() != 0
    }

    /// Sets the test-only class flag.
    pub fn set_is_test_only(&mut self, is_test_only: bool) {
        self.0.set_isTestOnly(is_test_only as u32)
    }

    /// Returns whether the class is marked savable.
    pub fn is_savable(&self) -> bool {
        self.0.isSavable() != 0
    }

    /// Sets whether the class is marked savable.
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

/// Rust-defined native [`Class`] with construction, cloning, and destruction callbacks.
#[derive(Debug)]
#[repr(transparent)]
pub struct NativeClass<T>(Class, PhantomData<*mut T>);

impl<T> NativeClass<T> {
    /// Creates a new native class with the given base type.
    /// Returns a handle to the class, this handle should be used to register the class with
    /// [`RttiSystemMut`](crate::RttiSystemMut). Any further interaction with the class should be
    /// done through RTTI.
    pub fn new_handle(name: &CStr, base: Option<&Class>) -> ClassHandle
    where
        T: Default + Clone,
    {
        const VFT_SIZE: usize = 30;
        const IS_EQUAL_SLOT: usize = 9;
        const ASSIGN_SLOT: usize = 10;
        const CONSTRUCT_SLOT: usize = 27;
        const DESTRUCT_SLOT: usize = 28;
        const ALLOC_SLOT: usize = 29;

        let size = u32::try_from(mem::size_of::<T>()).expect("native class size exceeds u32");
        let alignment =
            u32::try_from(mem::align_of::<T>()).expect("native class alignment exceeds u32");
        let mut class = Class::new_native(name, size);
        class.0.alignment = alignment;
        if let Some(base) = base {
            class.0.parent = base.as_raw() as *const _ as *mut red::CClass;
        }

        let vft = class.as_raw()._base.vtable_ as *mut usize;
        let vft = unsafe { slice::from_raw_parts(vft, VFT_SIZE) };
        let mut vft = vft.to_vec();
        vft[IS_EQUAL_SLOT] = Self::is_equal as *const () as _;
        vft[ASSIGN_SLOT] = Self::assign as *const () as _;
        vft[CONSTRUCT_SLOT] = Self::construct as *const () as _;
        vft[DESTRUCT_SLOT] = Self::destruct as *const () as _;
        vft[ALLOC_SLOT] = Self::alloc as *const () as _;

        class.0._base.vtable_ = vft.leak().as_ptr() as _;

        // we leak the class and wrap it as pointer, because RTTI expects all references to it
        // to live forever - this prevents accidental misuse
        ClassHandle(NonNull::from(Box::leak(Box::new(class))))
    }

    unsafe extern "C" fn is_equal(this: VoidPtr, lhs: VoidPtr, rhs: VoidPtr, unk: u32) -> bool {
        unsafe {
            crate::fn_from_hash!(
                TTypedClass_IsEqual,
                unsafe extern "C" fn(VoidPtr, VoidPtr, VoidPtr, u32) -> bool
            )(this, lhs, rhs, unk)
        }
    }

    // Retains the existing initialized-destination contract. The SDK's use of
    // placement new alone does not establish destination lifetime at call sites.
    unsafe extern "C" fn assign(_this: *const Self, lhs: *mut T, rhs: *const T)
    where
        T: Clone,
    {
        // Rust references must not be created for a self-assignment alias.
        if lhs.cast_const() != rhs {
            unsafe { (&mut *lhs).clone_from(&*rhs) };
        }
    }

    unsafe extern "C" fn construct(_this: *const Self, mem: *mut T)
    where
        T: Default,
    {
        unsafe {
            ptr::write(mem, T::default());
        }
    }

    unsafe extern "C" fn destruct(_this: *const Self, mem: *mut T) {
        unsafe {
            ptr::drop_in_place(mem);
        }
    }

    unsafe extern "C" fn alloc(this: *const Self) -> *mut T {
        let this = unsafe { &*this };
        let align = this.0.as_type().alignment();
        let size = this.0.as_type().size().next_multiple_of(align);
        let alloc: *mut T = unsafe { this.0.as_type().allocator().alloc_aligned(size, align) };
        alloc
    }
}

/// Class handle to be used to register a class with [`RttiSystemMut`](crate::RttiSystemMut).
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

/// Metadata for a raw pointer to a reflected type.
///
/// SDK: [CRTTIPointerType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct PointerType(red::CRTTIPointerType);

impl PointerType {
    /// Returns metadata for the referenced type.
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a strong, reference-counted object handle.
///
/// SDK: [CRTTIHandleType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct RefType(red::CRTTIHandleType);

impl RefType {
    /// Returns metadata for the referenced type.
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a weak object handle.
///
/// SDK: [CRTTIWeakHandleType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct WeakRefType(red::CRTTIWeakHandleType);

impl WeakRefType {
    /// Returns metadata for the referenced type.
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a script reference to a reflected value.
///
/// SDK: [CRTTIScriptReferenceType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct ScriptRefType(red::CRTTIScriptReferenceType);

impl ScriptRefType {
    /// Returns metadata for the referenced type.
    #[inline]
    pub fn pointee(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a static array, including its element type and fixed size.
///
/// SDK: [CRTTIStaticArrayType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct StaticArrayType(red::CRTTIStaticArrayType);

impl StaticArrayType {
    /// Returns metadata for the element value type.
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0._base.innerType.cast::<Type>() }
    }

    /// Returns the fixed element count, not the byte size of a value.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size as _
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a native array, including its element type and fixed size.
///
/// SDK: [CRTTINativeArrayType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct NativeArrayType(red::CRTTINativeArrayType);

impl NativeArrayType {
    /// Returns metadata for the element value type.
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0._base.innerType.cast::<Type>() }
    }

    /// Returns the fixed element count, not the byte size of a value.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size as _
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a resource reference and its resource type.
///
/// SDK: [CRTTIResourceReferenceType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct ResourceRefType(red::CRTTIResourceReferenceType);

impl ResourceRefType {
    /// Returns metadata for the referenced resource type.
    #[inline]
    pub fn resource_type(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for an asynchronous resource reference and its resource type.
///
/// SDK: [CRTTIResourceAsyncReferenceType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct RaRefType(red::CRTTIResourceAsyncReferenceType);

impl RaRefType {
    /// Returns metadata for the referenced resource type.
    #[inline]
    pub fn resource_type(&self) -> &Type {
        unsafe { &*self.0.innerType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Metadata for a legacy single-channel curve and its value type.
///
/// SDK: [CRTTILegacySingleChannelCurveType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct CurveType(red::CRTTILegacySingleChannelCurveType);

impl CurveType {
    /// Returns metadata for the element value type.
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*self.0.curveType.cast::<Type>() }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type_mut(&mut self) -> &mut Type {
        unsafe { &mut *(self as *mut _ as *mut Type) }
    }
}

/// Reflected function signature and execution metadata shared by globals and methods.
///
/// SDK: [CBaseFunction](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Function(red::CBaseFunction);

impl Function {
    /// Returns the full function name used for overload resolution.
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.fullName)
    }

    /// Returns the function name without its overload signature.
    #[inline]
    pub fn short_name(&self) -> CName {
        CName::from_raw(self.0.shortName)
    }

    /// Returns a copy of the flags; use `set_flags` to apply changes.
    #[inline]
    pub fn flags(&self) -> FunctionFlags {
        FunctionFlags(self.0.flags)
    }

    /// Replaces the stored flags with `flags`.
    #[inline]
    pub fn set_flags(&mut self, flags: FunctionFlags) {
        self.0.flags = flags.0;
    }

    /// Returns the owning class, or `None` for a global function.
    #[inline]
    pub fn parent(&self) -> Option<&Class> {
        unsafe { (self.vft().get_parent)(self).as_ref() }
    }

    /// Returns metadata for local variables.
    #[inline]
    pub fn locals(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.localVars) }
    }

    /// Returns parameter metadata in declaration order.
    #[inline]
    pub fn params(&self) -> &RedArray<&Property> {
        unsafe { mem::transmute(&self.0.params) }
    }

    /// Returns return-value metadata, or `None` for a void function.
    #[inline]
    pub fn return_value(&self) -> Option<&Property> {
        unsafe { (self.0.returnType.cast::<Property>()).as_ref() }
    }

    /// Appends a parameter by registered RTTI type name.
    ///
    /// `is_out` marks an output parameter; `is_optional` permits omission by script callers.
    /// Returns `false` if the type cannot be resolved or property allocation fails.
    /// Requires initialized RTTI; script loading may replace this metadata.
    #[inline]
    pub fn add_param(&mut self, typ: CName, name: &CStr, is_out: bool, is_optional: bool) -> bool {
        unsafe {
            self.0
                .AddParam(typ.to_raw(), name.as_ptr(), is_out, is_optional)
        }
    }

    /// Sets return-value metadata by registered RTTI type name.
    ///
    /// An unresolved type or allocation failure leaves the existing metadata unchanged;
    /// this wrapper does not report failure. Requires initialized RTTI, and script
    /// loading may replace this metadata.
    #[inline]
    pub fn set_return_type(&mut self, typ: CName) {
        unsafe { self.0.SetReturnType(typ.to_raw()) };
    }

    /// Executes with a tuple of [`NativeRepr`] arguments and a matching return type.
    /// Requires initialized engine RTTI and a valid receiver for instance methods.
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
            log::error!("Call error: {err}");
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
                return Err(InvokeError::ArgMismatch {
                    function: self.name().as_str(),
                    expected: param.type_().name().as_str(),
                    index,
                });
            }
        }

        match (ret.type_(), self.return_value()) {
            (None, None) => {}
            (Some(given), Some(expected)) if ptr::eq(given, expected.type_()) => {}
            _ => {
                return Err(InvokeError::ReturnMismatch {
                    function: self.name().as_str(),
                    expected: self
                        .return_value()
                        .map_or("Void", |ret| ret.type_().name().as_str()),
                });
            }
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

/// A reflected global function with no owning class.
///
/// SDK: [CGlobalFunction](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct GlobalFunction(red::CGlobalFunction);

impl GlobalFunction {
    /// Allocates native global-function metadata with the supplied names and handler.
    ///
    /// Requires initialized engine systems. Configure the matching parameters and return
    /// type before registration; this does not register the function with RTTI.
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

    /// Borrows the shared function metadata.
    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    /// Mutably borrows the shared function metadata.
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

/// A reflected instance method belonging to a class.
///
/// SDK: [CClassFunction](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Method(red::CClassFunction);

impl Method {
    /// Allocates native instance-method metadata for `class`.
    ///
    /// The class name must match `C::NAME`. Requires initialized engine systems; configure
    /// the matching signature and attach the method to its class separately.
    pub fn new<C, R>(
        full_name: &CStr,
        short_name: &CStr,
        class: &Class,
        handler: FunctionHandler<C, R>,
        flags: FunctionFlags,
    ) -> PoolRef<Self>
    where
        C: ScriptClass,
    {
        let mut func = Method::alloc().expect("should allocate a Method");
        let full_name = CNamePool::add_cstr(full_name);
        let short_name = CNamePool::add_cstr(short_name);

        check_invariant(
            CName::new(C::NAME) == class.name(),
            "class name doesn't match",
        );

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

    /// Borrows the shared function metadata.
    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    /// Mutably borrows the shared function metadata.
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

/// A reflected class method that does not require an instance receiver.
///
/// SDK: [CClassStaticFunction](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct StaticMethod(red::CClassStaticFunction);

impl StaticMethod {
    /// Allocates native static-method metadata for `class`.
    ///
    /// Requires initialized engine systems. Configure the matching signature and attach
    /// the method to its class separately.
    pub fn new<R>(
        full_name: &CStr,
        short_name: &CStr,
        class: &Class,
        handler: FunctionHandler<IScriptable, R>,
        flags: FunctionFlags,
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
            ctor(ptr, class, full_name, short_name, handler, flags.0);
        };
    }

    /// Borrows the shared function metadata.
    #[inline]
    pub fn as_function(&self) -> &Function {
        unsafe { &*(self as *const _ as *const Function) }
    }

    /// Mutably borrows the shared function metadata.
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

/// Function implementation and dispatch flags.
///
/// Getters on [`Function`] return a copy; apply changes with [`Function::set_flags`].
/// Setting flags does not install a handler or change its calling convention.
///
/// SDK: [CBaseFunction flags](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct FunctionFlags(red::CBaseFunction_Flags);

impl FunctionFlags {
    /// Returns whether execution uses a native handler rather than script bytecode.
    pub fn is_native(&self) -> bool {
        self.0.isNative() != 0
    }

    /// Sets the native dispatch flag without installing or changing the handler.
    pub fn set_is_native(&mut self, is_native: bool) {
        self.0.set_isNative(is_native as u32)
    }

    /// Returns whether the function is static and needs no instance receiver.
    pub fn is_static(&self) -> bool {
        self.0.isStatic() != 0
    }

    /// Sets the static dispatch flag.
    pub fn set_is_static(&mut self, is_static: bool) {
        self.0.set_isStatic(is_static as u32)
    }

    /// Returns whether the function is marked final (not overridable).
    pub fn is_final(&self) -> bool {
        self.0.isFinal() != 0
    }

    /// Sets whether the function is marked final.
    pub fn set_is_final(&mut self, is_final: bool) {
        self.0.set_isFinal(is_final as u32)
    }

    /// Returns whether the function is declared as an event.
    pub fn is_event(&self) -> bool {
        self.0.isEvent() != 0
    }

    /// Sets the event function flag.
    pub fn set_is_event(&mut self, is_event: bool) {
        self.0.set_isEvent(is_event as u32)
    }
}

/// Metadata for a class field, function parameter, local, or return value.
///
/// SDK: [CProperty](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/CProperty.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Property(red::CProperty);

impl Property {
    /// Allocates property metadata with default flags and a byte offset.
    ///
    /// `group` names the property group. The parent and type metadata must outlive
    /// the property. This neither registers the property nor initializes its storage.
    /// Set the holder-relative flag separately when the offset uses a script value holder.
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
            (*ptr).0.type_ = type_.as_raw() as *const _ as *mut red::rtti::IType;
            (*ptr).0.parent = parent.as_raw() as *const _ as *mut red::CClass;
            (*ptr).0.valueOffset = offset;
            (*ptr).0.flags = PropertyFlags::default().0;
            prop.assume_init()
        }
    }

    /// Returns the property name.
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    /// Returns a copy of the flags; use `set_flags` to apply changes.
    #[inline]
    pub fn flags(&self) -> PropertyFlags {
        PropertyFlags(self.0.flags)
    }

    /// Replaces the stored flags with `flags`.
    #[inline]
    pub fn set_flags(&mut self, flags: PropertyFlags) {
        self.0.flags = flags.0;
    }

    /// Returns metadata for the stored value type.
    #[inline]
    pub fn type_(&self) -> &Type {
        unsafe { &*(self.0.type_ as *const Type) }
    }

    /// Returns the byte offset from the storage base indicated by the property flags.
    /// See [`PropertyFlags::in_value_holder`] when choosing a container for [`Self::value`].
    #[inline]
    pub fn value_offset(&self) -> u32 {
        self.0.valueOffset
    }

    /// Returns the address at [`Self::value_offset`] bytes from `container`.
    ///
    /// This only adds the offset: it does not inspect flags or select a value holder.
    /// For [`PropertyFlags::in_value_holder`], pass the object's [`IScriptable::fields`]
    /// container; otherwise pass the appropriate native object or argument-storage base.
    ///
    /// # Safety
    /// `container` must be a live storage base matching this property's layout and
    /// flags. The offset must stay within its allocation and address an initialized,
    /// properly aligned value of [`Self::type_`]. The returned pointer must not be
    /// used after the storage is destroyed or relocated, or in violation of aliasing rules.
    #[inline]
    pub unsafe fn value(&self, container: ValueContainer) -> ValuePtr {
        unsafe { ValuePtr(container.0.byte_add(self.0.valueOffset as usize)) }
    }
}

/// Property origin and storage-location flags.
///
/// Getters on [`Property`] return a copy; apply changes with [`Property::set_flags`].
/// Setting these flags does not move or initialize the property's value.
///
/// SDK: [CProperty flags](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/CProperty.hpp).
#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct PropertyFlags(red::CProperty_Flags);

impl PropertyFlags {
    /// Returns whether the property is script-defined.
    pub fn is_scripted(&self) -> bool {
        self.0.isScripted() != 0
    }

    /// Sets the script-defined property flag; this does not select its storage base.
    pub fn set_is_scripted(&mut self, is_scripted: bool) {
        self.0.set_isScripted(is_scripted as u64)
    }

    /// Returns whether the offset is relative to an `IScriptable` value holder,
    /// rather than the native object address.
    ///
    /// [`Property::value`] does not select this holder automatically; callers must
    /// pass the appropriate [`ValueContainer`], such as [`IScriptable::fields`].
    pub fn in_value_holder(&self) -> bool {
        self.0.inValueHolder() != 0
    }

    /// Sets whether the property offset is holder-relative rather than object-relative.
    /// This changes metadata only; callers of [`Property::value`] still choose the container.
    pub fn set_in_value_holder(&mut self, in_value_holder: bool) {
        self.0.set_inValueHolder(in_value_holder as u64)
    }
}

/// Common array metadata and engine operations for inspecting array values.
///
/// SDK: [CRTTIBaseArrayType](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct ArrayType(red::CRTTIBaseArrayType);

impl ArrayType {
    /// Returns metadata for the element value type.
    #[inline]
    pub fn element_type(&self) -> &Type {
        unsafe { &*(self.vft().get_inner_type)(self) }
    }

    /// Returns the number of elements in an array value.
    ///
    /// # Safety
    /// `val` must point to a live, aligned, initialized array of this exact RTTI type.
    /// Its storage must remain readable and must not be concurrently mutated or destroyed.
    #[inline]
    pub unsafe fn length(&self, val: ValuePtr) -> u32 {
        unsafe { (self.vft().get_length)(self, val) }
    }

    /// Returns a pointer to an element without checking bounds.
    ///
    /// # Safety
    /// `val` must point to a live, aligned, initialized array of this exact RTTI type,
    /// and `index` must be less than its current length. Keep its storage alive and
    /// stable while using the result; mutation or resizing may invalidate it.
    /// Access through the result must respect the element type and aliasing rules.
    #[inline]
    pub unsafe fn element(&self, val: ValuePtr, index: u32) -> ValuePtr {
        unsafe { (self.vft().get_element)(self, val, index) }
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
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

/// Enumeration metadata with parallel lists of variant names and integer values.
///
/// SDK: [CEnum](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Enum(red::CEnum);

impl Enum {
    /// Returns the reflected enum name.
    #[inline]
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    /// Returns variant names, indexed in parallel with [`Self::variant_values`].
    #[inline]
    pub fn variant_names(&self) -> &RedArray<CName> {
        unsafe { mem::transmute(&self.0.hashList) }
    }

    /// Returns integer values, indexed in parallel with [`Self::variant_names`].
    #[inline]
    pub fn variant_values(&self) -> &RedArray<i64> {
        unsafe { mem::transmute(&self.0.valueList) }
    }

    /// Returns the size of a value in bytes.
    #[inline]
    pub fn byte_size(&self) -> u8 {
        self.0.actualSize
    }

    /// Borrows the common RTTI metadata for this type.
    #[inline]
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
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

/// Bitfield metadata with names indexed by bit position.
///
/// SDK: [CBitfield](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct Bitfield(red::CBitfield);

impl Bitfield {
    /// Returns the reflected bitfield name.
    pub fn name(&self) -> CName {
        CName::from_raw(self.0.name)
    }

    /// Returns the size of a value in bytes.
    pub fn byte_size(&self) -> u8 {
        self.0.actualSize
    }

    /// Returns the 64 name slots indexed by bit position; unused slots may be undefined.
    pub fn fields(&self) -> &[CName; 64] {
        unsafe { mem::transmute(&self.0.bitNames) }
    }

    /// Borrows the common RTTI metadata for this type.
    pub fn as_type(&self) -> &Type {
        unsafe { &*(self as *const _ as *const Type) }
    }

    /// Mutably borrows the common RTTI metadata for this type.
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

/// Base object for handle-managed instances, providing runtime type and identity queries.
///
/// SDK: [ISerializable](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/ISerializable.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct ISerializable(red::ISerializable);

impl ISerializable {
    /// Returns the runtime class, which may be more derived than the native base.
    #[inline]
    pub fn class(&self) -> &Class {
        unsafe {
            &*(((*self.0.vtable_).ISerializable_GetType)(
                (&self.0) as *const _ as *mut red::ISerializable,
            ) as *const Class)
        }
    }

    /// Returns whether the runtime class is `U` or derives from it, comparing RTTI names.
    #[inline]
    pub fn is_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        self.class()
            .base_iter_with_self()
            .any(|class| class.name() == CName::new(U::NAME))
    }

    /// Returns whether the runtime class name is exactly `U::NAME`, excluding subclasses.
    #[inline]
    pub fn is_exactly_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        self.class().name() == CName::new(U::NAME)
    }

    /// Clones the embedded weak handle if the object is `U` or derives from it.
    /// This does not acquire a strong reference or keep the object alive.
    #[inline]
    pub fn inner_ref<U>(&self) -> Option<WeakRef<U>>
    where
        U: ScriptClass,
    {
        self.is_a::<U>().then(|| {
            unsafe {
                mem::transmute::<&red::WeakHandle<red::ISerializable>, &WeakRef<U>>(&self.0.ref_)
            }
            .clone()
        })
    }

    /// Borrows the native virtual-function table.
    #[inline]
    pub fn vft(&self) -> &ISerializableVft {
        unsafe { &*(self.0.vtable_ as *const ISerializableVft) }
    }
}

impl PtrEq for ISerializable {
    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ref_._base._base.ptr_eq(&other.0.ref_._base._base)
    }
}

unsafe impl ScriptClass for ISerializable {
    type Kind = class_kind::Native;

    const NAME: &'static str = "ISerializable";
}

/// Scriptable object base with separate value-holder storage for scripted fields.
///
/// SDK: [IScriptable](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/IScriptable.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct IScriptable(red::IScriptable);

impl<T: AsRef<IScriptable>, U: AsRef<IScriptable>> PtrEq<U> for T {
    fn ptr_eq(&self, other: &U) -> bool {
        self.as_ref()
            .as_serializable()
            .ptr_eq(other.as_ref().as_serializable())
    }
}

impl IScriptable {
    /// Returns the runtime class, including script-defined subclasses.
    #[inline]
    pub fn class(&self) -> &Class {
        self.as_serializable().class()
    }

    /// Returns the separate script value-holder base, which may be null.
    ///
    /// Use this container for holder-relative properties, not native object-relative
    /// properties. It does not keep the object or holder alive.
    #[inline]
    pub fn fields(&self) -> ValueContainer {
        ValueContainer(self.0.valueHolder)
    }

    /// Borrows the handle-managed object base.
    #[inline]
    pub fn as_serializable(&self) -> &ISerializable {
        unsafe { &*(self as *const _ as *const ISerializable) }
    }

    /// Replaces the native class metadata pointer without changing object layout.
    /// The class metadata must remain valid for the lifetime of this object.
    #[inline]
    pub fn set_native_type(&mut self, class: &Class) {
        self.0.nativeType = class.as_raw() as *const _ as *mut red::CClass;
    }

    /// Creates a fresh native base, retaining type metadata but not object identity.
    ///
    /// This deliberately does not copy scripted fields or the embedded weak
    /// handle. Native Rust classes can use this in a manual `Clone` implementation
    /// to copy their own fields without duplicating engine-owned base state.
    /// Scripted objects must be copied through engine RTTI instead.
    pub fn clone_native_base(&self) -> Self {
        assert!(
            self.0.valueHolder.is_null(),
            "cannot copy a scripted value holder"
        );
        if let Some(class) = unsafe { self.0.nativeType.cast::<Class>().as_ref() } {
            assert!(
                !class.flags().is_scripted_class() && !class.flags().is_scripted_struct(),
                "cannot copy a scripted object as a native base"
            );
        }
        let mut base = Self::default();
        // Keep the fresh base's vtable: a derived vtable may access fields that
        // are not present in this standalone IScriptable value.
        base.0.nativeType = self.0.nativeType;
        base
    }

    /// Borrows the native virtual-function table.
    #[inline]
    pub fn vft(&self) -> &IScriptableVft {
        unsafe { &*(self.0._base.vtable_ as *const IScriptableVft) }
    }
}

impl Clone for IScriptable {
    fn clone(&self) -> Self {
        self.clone_native_base()
    }

    /// Never copy an object's weak handle or value holder as raw bytes.
    fn clone_from(&mut self, _: &Self) {}
}

impl Default for IScriptable {
    #[inline]
    fn default() -> Self {
        Self(unsafe { red::IScriptable::new() })
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
    type Kind = class_kind::Native;

    const NAME: &'static str = "IScriptable";
}

/// A non-owning base address for values, such as object fields or function arguments.
///
/// This copyable wrapper does not keep storage alive or encode its layout or lifetime.
/// Property access requires the correct base: native object storage or, for holder-relative
/// properties, the value holder returned by [`IScriptable::fields`].
#[derive(Debug, Clone, Copy)]
pub struct ValueContainer(VoidPtr);

impl ValueContainer {
    #[inline]
    pub(super) fn new(ptr: VoidPtr) -> Self {
        Self(ptr)
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> VoidPtr {
        self.0
    }
}

/// A non-owning pointer to a reflected value's storage.
///
/// This wrapper does not encode the value's type, extend its lifetime, or prevent
/// native aliasing. It may point to handle storage rather than directly to an object.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ValuePtr(VoidPtr);

impl ValuePtr {
    /// Reads strong object-handle storage, returning `None` for an empty or expired handle.
    ///
    /// # Safety
    /// This pointer must address live, aligned, initialized strong-handle storage
    /// compatible with `IScriptable`, not a raw object pointer or weak handle.
    /// Its reference count must remain valid while inspected, and the object must
    /// remain alive and free of conflicting mutation for the returned borrow.
    /// The strong-count check does not acquire ownership or prevent native races.
    pub unsafe fn unwrap_ref(&self) -> Option<&IScriptable> {
        let ptr = self.0 as *mut red::SharedPtrBase<red::IScriptable>;
        unsafe {
            let inst = (*ptr).instance;
            let rc = (*ptr).refCount;
            if inst.is_null() || rc.is_null() || (*rc).strongRefs == 0 {
                return None;
            };
            Some(&*(inst as *const IScriptable))
        }
    }

    /// Uses this value's storage address as a base for nested property offsets.
    ///
    /// This does not dereference an object handle or select a script value holder.
    ///
    /// # Safety
    /// The pointer must be a valid base for the intended container layout and
    /// remain live for every subsequent access through the returned container.
    #[inline]
    pub unsafe fn to_container(&self) -> ValueContainer {
        ValueContainer(self.0)
    }
}

#[repr(C)]
struct TypeVft {
    destroy: unsafe extern "C" fn(this: *mut Type),
    tail: red::rtti::IType__bindgen_vtable,
}

#[repr(C)]
struct ArrayTypeVft {
    base: TypeVft,
    get_inner_type: unsafe extern "C" fn(this: *const ArrayType) -> *const Type,
    sub_c8: unsafe extern "C" fn(this: *const ArrayType) -> bool,
    get_length: unsafe extern "C" fn(this: *const ArrayType, val: ValuePtr) -> u32,
    get_max_length: unsafe extern "C" fn(this: *const ArrayType) -> u32,
    get_element:
        unsafe extern "C" fn(this: *const ArrayType, val: ValuePtr, index: u32) -> ValuePtr,
}

#[repr(C)]
struct FunctionVft {
    get_allocator: unsafe extern "C" fn(this: &Function) -> *mut IAllocator,
    destruct: unsafe extern "C" fn(this: &mut Function),
    get_parent: unsafe extern "C" fn(this: &Function) -> *mut Class,
}

/// Native virtual-function table layout for [`ISerializable`].
#[repr(C)]
pub struct ISerializableVft {
    get_native_type: unsafe extern "C" fn(this: &ISerializable) -> *mut Class,
    get_type: unsafe extern "C" fn(this: &ISerializable) -> *mut Class,
    get_allocator: unsafe extern "C" fn(this: &ISerializable) -> *mut IAllocator,
    destroy: unsafe extern "C" fn(this: *mut ISerializable),
    sub_20: VoidPtr,
    post_load: VoidPtr,
    sub_30: VoidPtr,
    sub_38: VoidPtr,
    sub_40: VoidPtr,
    sub_48: VoidPtr,
    sub_50: VoidPtr,
    sub_58: VoidPtr,
    sub_60: VoidPtr,
    sub_68: VoidPtr,
    sub_70: VoidPtr,
    sub_78: VoidPtr,
    sub_80: VoidPtr,
    sub_88: VoidPtr,
    sub_90: VoidPtr,
    sub_98: VoidPtr,
    sub_a0: VoidPtr,
    sub_a8: VoidPtr,
    sub_b0: VoidPtr,
    sub_b8: VoidPtr,
    sub_c0: VoidPtr,
    sub_c8: VoidPtr,
    can_be_destructed: unsafe extern "C" fn(this: VoidPtr) -> bool,
}

/// Native virtual-function table layout for [`IScriptable`].
#[repr(C)]
pub struct IScriptableVft {
    base: ISerializableVft,
    sub_d8: VoidPtr,
    sub_e0: VoidPtr,
    sub_e8: VoidPtr,
    sub_f0: VoidPtr,
    sub_f8: VoidPtr,
    sub_100: VoidPtr,
}

#[cfg(test)]
mod native_class_tests {
    use super::*;

    #[test]
    fn trampolines_have_explicit_c_signatures() {
        let _: unsafe extern "C" fn(VoidPtr, VoidPtr, VoidPtr, u32) -> bool =
            NativeClass::<u64>::is_equal;
        let _: unsafe extern "C" fn(*const NativeClass<u64>, *mut u64, *const u64) =
            NativeClass::<u64>::assign;
        let _: unsafe extern "C" fn(*const NativeClass<u64>, *mut u64) =
            NativeClass::<u64>::construct;
        let _: unsafe extern "C" fn(*const NativeClass<u64>, *mut u64) =
            NativeClass::<u64>::destruct;
        let _: unsafe extern "C" fn(*const NativeClass<u64>) -> *mut u64 =
            NativeClass::<u64>::alloc;
    }

    #[test]
    fn assignment_handles_self_alias_without_references() {
        let mut value = String::from("unchanged");
        let value_ptr = ptr::from_mut(&mut value);
        unsafe { NativeClass::<String>::assign(ptr::null(), value_ptr, value_ptr) };
        assert_eq!(value, "unchanged");
    }

    #[test]
    fn construction_initializes_raw_storage() {
        let mut value = mem::MaybeUninit::<u64>::uninit();
        unsafe { NativeClass::<u64>::construct(ptr::null(), value.as_mut_ptr()) };
        assert_eq!(unsafe { value.assume_init() }, 0);
    }
}
