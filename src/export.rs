use std::ffi::CString;
use std::marker::PhantomData;

use sealed::sealed;

use crate::invocable::{GlobalMetadata, MethodMetadata};
use crate::systems::RttiSystemMut;
use crate::types::{CName, NativeClass};
use crate::{NativeRepr, RttiSystem, ScriptClass, class_kind};

/// A list of exports to register with the game.
#[derive(Debug)]
pub struct ExportList<H, T> {
    head: H,
    tail: T,
}

impl<H, T> ExportList<H, T> {
    /// Create a new `ExportList` with the given head and tail.
    pub const fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

/// Registers definitions in the SDK's
/// [RTTI callbacks](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTISystem.hpp).
/// Normally driven by [`Plugin::exports`](crate::Plugin::exports), not called manually.
#[sealed]
pub trait Exportable {
    fn register(&self);
    fn post_register(&self);
}

#[sealed]
impl<H, T> Exportable for ExportList<H, T>
where
    H: Exportable,
    T: Exportable,
{
    #[inline]
    fn register(&self) {
        self.head.register();
        self.tail.register();
    }

    #[inline]
    fn post_register(&self) {
        self.head.post_register();
        self.tail.post_register();
    }
}

/// A type representing an empty list of exports.
#[derive(Debug)]
pub struct ExportNil;

#[sealed]
impl Exportable for ExportNil {
    #[inline]
    fn register(&self) {}

    #[inline]
    fn post_register(&self) {}
}

/// Registers a native class backed by SDK
/// [CClass](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
/// Include the base class in the Rust layout. For value types, use [`StructExport`].
/// Implement `Clone` explicitly: use `IScriptable::clone_native_base` for a fresh
/// object's base and preserve the destination base in `clone_from`. Never copy
/// an object's weak handle or value holder as raw bytes.
#[derive(Debug)]
pub struct ClassExport<C: 'static> {
    base: &'static str,
    methods: &'static [MethodMetadata<C>],
    static_methods: &'static [GlobalMetadata],
}

impl<C: ScriptClass> ClassExport<C> {
    pub fn builder() -> ClassExportBuilder<C> {
        ClassExportBuilder {
            base: "IScriptable",
            methods: &[],
            static_methods: &[],
        }
    }
}

#[sealed]
impl<C: Default + Clone + ScriptClass<Kind = class_kind::Native>> Exportable for ClassExport<C> {
    fn register(&self) {
        let mut rtti = RttiSystemMut::get();
        let name_cstr = CString::new(C::NAME).expect("name should be valid");
        let base = rtti
            .get_class(CName::new(self.base))
            .expect("base should exist");
        let handle = NativeClass::<C>::new_handle(&name_cstr, Some(base));
        rtti.register_class(handle);
    }

    fn post_register(&self) {
        let (converted_methods, converted_static_methods) = {
            let rtti_ro = RttiSystem::get();
            let class = rtti_ro
                .get_class(CName::new(C::NAME))
                .expect("class should exist");
            // macOS: the conversion below calls `Function::add_param` / `set_return_type`, which ask the game's `GetType` for the parameter
            // types, and `GetType` of a composite name that is not registered yet (an array or static array type, for example) creates
            // the type and registers it under the exclusive types lock. The shared guard this block holds would never be released
            // (B2 report, Mac `GetType` 0x21927A8 -> 0x2192F00 -> `RegisterType` 0x21920B4: lock byte 0 to 0x80 while this thread has +1 on it),
            // so the registration would hang. A class is never freed, so the guard is released before the conversion.
            #[cfg(target_os = "macos")]
            let class = {
                let class: *const crate::types::Class = class;
                drop(rtti_ro);
                // SAFETY: registered classes live as long as the RTTI system.
                unsafe { &*class }
            };
            let converted_methods = self
                .methods
                .iter()
                .map(|m| m.to_rtti(class))
                .collect::<Vec<_>>();
            let converted_static_methods = self
                .static_methods
                .iter()
                .map(|m| m.to_rtti_static_method(class))
                .collect::<Vec<_>>();
            (converted_methods, converted_static_methods)
        };

        let mut rtti_rw = RttiSystemMut::get();
        let class = rtti_rw
            .get_class(CName::new(C::NAME))
            .expect("class should exist");

        for method in converted_methods {
            class.add_method(method);
        }
        for static_method in converted_static_methods {
            class.add_static_method(static_method);
        }
    }
}

/// A builder for [`ClassExport`].
#[derive(Debug)]
pub struct ClassExportBuilder<C: 'static> {
    base: &'static str,
    methods: &'static [MethodMetadata<C>],
    static_methods: &'static [GlobalMetadata],
}

impl<C> ClassExportBuilder<C> {
    /// Set the base class of the class to be exported.
    /// This is set to `IScriptable` by default.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = base;
        self
    }

    /// Set the methods of the class to be exported.
    /// See the [`methods!`](crate::methods) macro for a convenient way to define methods.
    pub const fn methods(mut self, methods: &'static [MethodMetadata<C>]) -> Self {
        self.methods = methods;
        self
    }

    /// Set the static methods of the class to be exported.
    /// See the [`static_methods!`](crate::static_methods) macro for a convenient way to define methods.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Build the final [`ClassExport`] instance.
    pub const fn build(self) -> ClassExport<C> {
        ClassExport {
            base: self.base,
            methods: self.methods,
            static_methods: self.static_methods,
        }
    }
}

/// Registers a native value type. Unlike [`ClassExport`], it has no default base class.
#[derive(Debug)]
pub struct StructExport<C> {
    base: Option<&'static str>,
    static_methods: &'static [GlobalMetadata],
    _phantom: PhantomData<fn() -> C>,
}

impl<C> StructExport<C> {
    pub fn builder() -> StructExportBuilder<C> {
        StructExportBuilder {
            base: None,
            static_methods: &[],
            _phantom: PhantomData,
        }
    }
}

#[sealed]
impl<C: Default + Clone + NativeRepr> Exportable for StructExport<C> {
    fn register(&self) {
        let mut rtti = RttiSystemMut::get();
        let name_cstr = CString::new(C::NAME).expect("name should be valid");
        let base = self
            .base
            .map(|base| &*rtti.get_class(CName::new(base)).expect("base should exist"));
        let handle = NativeClass::<C>::new_handle(&name_cstr, base);
        rtti.register_class(handle);
    }

    fn post_register(&self) {
        let converted_static_methods = {
            let rtti_ro = RttiSystem::get();
            let class = rtti_ro
                .get_class(CName::new(C::NAME))
                .expect("class should exist");
            // macOS: the shared guard is released before the conversion, see `ClassExport::post_register`.
            #[cfg(target_os = "macos")]
            let class = {
                let class: *const crate::types::Class = class;
                drop(rtti_ro);
                // SAFETY: registered classes live as long as the RTTI system.
                unsafe { &*class }
            };
            self.static_methods
                .iter()
                .map(|m| m.to_rtti_static_method(class))
                .collect::<Vec<_>>()
        };

        let mut rtti_rw = RttiSystemMut::get();
        let class = rtti_rw
            .get_class(CName::new(C::NAME))
            .expect("class should exist");

        for static_method in converted_static_methods {
            class.add_static_method(static_method);
        }
    }
}

/// A builder for [`StructExport`].
#[derive(Debug)]
pub struct StructExportBuilder<C> {
    base: Option<&'static str>,
    static_methods: &'static [GlobalMetadata],
    _phantom: PhantomData<fn() -> C>,
}

impl<C> StructExportBuilder<C> {
    /// Set the base type of the struct to be exported.
    /// Structs do not have a base type by default.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = Some(base);
        self
    }

    /// Set the static methods of the struct to be exported.
    /// See the [`static_methods!`](crate::static_methods) macro for a convenient way to define methods.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Build the final [`StructExport`] instance.
    pub const fn build(self) -> StructExport<C> {
        StructExport {
            base: self.base,
            static_methods: self.static_methods,
            _phantom: PhantomData,
        }
    }
}

/// A single global function export.
#[derive(Debug)]
pub struct GlobalExport(pub GlobalMetadata);

#[sealed]
impl Exportable for GlobalExport {
    #[inline]
    fn register(&self) {}

    fn post_register(&self) {
        let converted = self.0.to_rtti();

        let mut rtti = RttiSystemMut::get();
        rtti.register_function(converted);
    }
}

/// Creates a list of exports to be registered within the game's RTTI system.
///
/// # Example
/// ```rust
/// use std::cell::Cell;
///
/// use red4ext_rs::{ClassExport, Exportable, GlobalExport, ScriptClass, class_kind, exports, methods, global};
/// use red4ext_rs::types::IScriptable;
///
/// fn exports() -> impl Exportable {
///     exports![
///         GlobalExport(global!(c"GlobalExample", global_example)),
///         ClassExport::<MyClass>::builder()
///            .base("IScriptable")
///            .methods(methods![
///                c"Value" => MyClass::value,
///                c"SetValue" => MyClass::set_value,
///            ])
///            .build(),
///     ]
/// }
///
/// fn global_example() -> String {
///   "Hello, world!".to_string()
/// }
///
/// #[derive(Debug, Default)]
/// #[repr(C)]
/// struct MyClass {
///     // You must include the base native class in your Rust struct.
///     base: IScriptable,
///     value: Cell<i32>,
/// }
///
/// impl Clone for MyClass {
///     fn clone(&self) -> Self {
///         Self { base: self.base.clone_native_base(), value: self.value.clone() }
///     }
///
///     fn clone_from(&mut self, source: &Self) {
///         // Assignment preserves the destination's engine identity and holder.
///         self.value.set(source.value.get());
///     }
/// }
///
/// impl MyClass {
///    fn value(&self) -> i32 {
///       self.value.get()
///    }
///
///    fn set_value(&self, value: i32) {
///       self.value.set(value)
///    }
/// }
///
/// unsafe impl ScriptClass for MyClass {
///    const NAME: &'static str = "MyClass";
///    type Kind = class_kind::Native;
/// }
#[macro_export]
macro_rules! exports {
    [$export:expr, $($tt:tt)*] => {
        $crate::ExportList::new($export, $crate::exports!($($tt)*))
    };
    [$export:expr] => {
        $crate::ExportList::new($export, $crate::ExportNil)
    };
    [] => { $crate::ExportNil }
}

/// Defines instance methods, usually used with
/// [`exports!`].
#[macro_export]
macro_rules! methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::method!($($mod)* $name, $ty::$id)),*] }
    };
}

/// Defines static methods, usually used with
/// [`exports!`].
#[macro_export]
macro_rules! static_methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::global!($($mod)* $name, $ty::$id)),*] }
    };
}
