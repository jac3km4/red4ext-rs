use std::ffi::CString;
use std::marker::PhantomData;

use sealed::sealed;

use crate::invocable::{GlobalMetadata, MethodMetadata};
use crate::systems::RttiSystemMut;
use crate::types::{CName, NativeClass};
use crate::{NativeRepr, RttiSystem, ScriptClass, class_kind};

/// A list of classes and functions to register with the game; build with [`exports!`](crate::exports).
#[derive(Debug)]
pub struct ExportList<H, T> {
    head: H,
    tail: T,
}

impl<H, T> ExportList<H, T> {
    /// Prepends an export to the remaining list.
    pub const fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

/// Registers definitions during RTTI registration and post-registration.
/// Normally driven by [`Plugin::exports`](crate::Plugin::exports), not called manually.
///
/// SDK: [RTTI callbacks](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTISystem.hpp).
#[sealed]
pub trait Exportable {
    /// Registers type definitions during the RTTI registration callback.
    fn register(&self);
    /// Registers functions and methods after type definitions are available.
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

/// An empty export list, also returned by `exports![]`.
#[derive(Debug)]
pub struct ExportNil;

#[sealed]
impl Exportable for ExportNil {
    #[inline]
    fn register(&self) {}

    #[inline]
    fn post_register(&self) {}
}

/// Exports a native reference type and its instance and static methods as an RTTI class.
/// Include the base class in the Rust layout. For value types, use [`StructExport`].
/// Implement `Clone` explicitly: use `IScriptable::clone_native_base` for a fresh
/// object's base and preserve the destination base in `clone_from`. Never copy
/// an object's weak handle or value holder as raw bytes.
///
/// SDK: [CClass](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
#[derive(Debug)]
pub struct ClassExport<C: 'static> {
    base: &'static str,
    methods: &'static [MethodMetadata<C>],
    static_methods: &'static [GlobalMetadata],
}

impl<C: ScriptClass> ClassExport<C> {
    /// Starts a class export with base `IScriptable` and no methods.
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
    /// Sets the engine RTTI base class name (default: `IScriptable`).
    /// The Rust layout must include this base at the start of the object.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = base;
        self
    }

    /// Sets the instance methods, replacing any previous selection.
    /// Use [`methods!`](crate::methods) to build the slice.
    pub const fn methods(mut self, methods: &'static [MethodMetadata<C>]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets the static methods, replacing any previous selection.
    /// Use [`static_methods!`](crate::static_methods) to build the slice.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Builds the class export without registering it.
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
    /// Starts a value-type export with no base type or static methods.
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
    /// Sets the engine RTTI base type name; there is no base by default.
    /// The Rust layout must include the base at the start of the value.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = Some(base);
        self
    }

    /// Sets the static methods, replacing any previous selection.
    /// Use [`static_methods!`](crate::static_methods) to build the slice.
    pub const fn static_methods(mut self, static_methods: &'static [GlobalMetadata]) -> Self {
        self.static_methods = static_methods;
        self
    }

    /// Builds the value-type export without registering it.
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
pub struct GlobalExport(
    /// Function metadata created with [`global!`](crate::global).
    pub GlobalMetadata,
);

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

/// Builds an instance-method slice for [`ClassExportBuilder::methods`].
/// Entries use `c"Name" => Type::method`, optionally prefixed with `event` or `final`.
#[macro_export]
macro_rules! methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::method!($($mod)* $name, $ty::$id)),*] }
    };
}

/// Builds a static-method slice for a class or value-type export.
/// Entries use `c"Name" => Type::function`.
#[macro_export]
macro_rules! static_methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::global!($($mod)* $name, $ty::$id)),*] }
    };
}
