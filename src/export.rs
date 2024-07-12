use sealed::sealed;

use crate::invocable::{GlobalMetadata, MethodMetadata};
use crate::systems::RttiSystemMut;
use crate::types::{CName, NativeClass, ScriptClass};

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

/// A trait for types to be exported to the game.
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

/// A single class export.
/// This can be used to define a custom class to be exported to the game.
#[derive(Debug)]
pub struct ClassExport<C: 'static> {
    base: Option<&'static str>,
    methods: &'static [MethodMetadata<C>],
}

impl<C: ScriptClass> ClassExport<C> {
    pub fn builder() -> ClassExportBuilder<C> {
        ClassExportBuilder {
            base: None,
            methods: &[],
        }
    }
}

#[sealed]
impl<C: Default + Clone + ScriptClass> Exportable for ClassExport<C> {
    fn register(&self) {
        let mut rtti = RttiSystemMut::get();
        let base = self
            .base
            .map(|name| &*rtti.get_class(CName::new(name)).expect("base should exist"));
        let handle = NativeClass::<C>::new_handle(base);
        rtti.register_class(handle);
    }

    fn post_register(&self) {
        let converted = self
            .methods
            .iter()
            .map(MethodMetadata::to_rtti)
            .collect::<Vec<_>>();

        let mut rtti = RttiSystemMut::get();
        let class = rtti
            .get_class(CName::new(C::CLASS_NAME))
            .expect("class should exist");
        for method in converted {
            class.add_method(method);
        }
    }
}

/// A builder for [`ClassExport`].
#[derive(Debug)]
pub struct ClassExportBuilder<C: 'static> {
    base: Option<&'static str>,
    methods: &'static [MethodMetadata<C>],
}

impl<C> ClassExportBuilder<C> {
    /// Set the base class of the class to be exported.
    /// You must set this "IScriptable" or derived type to expose a class instead of a struct.
    /// You must include the base type as the first field in your struct.
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = Some(base);
        self
    }

    /// Set the methods of the class to be exported.
    /// See the [`methods!`] macro for a convenient way to define methods.
    pub const fn methods(mut self, methods: &'static [MethodMetadata<C>]) -> Self {
        self.methods = methods;
        self
    }

    /// Build the final [`ClassExport`] instance.
    pub const fn build(self) -> ClassExport<C> {
        ClassExport {
            base: self.base,
            methods: self.methods,
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

/// Define a list of exports to register with the game.
///
/// # Example
/// ```rust
/// use std::cell::Cell;
///
/// use red4rs::{ClassExport, Exportable, GlobalExport, exports, methods, global};
/// use red4rs::types::{IScriptable, ScriptClass, Native};
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
/// #[derive(Debug, Default, Clone)]
/// #[repr(C)]
/// struct MyClass {
///     // You must include the base native class in your struct.
///     base: IScriptable,
///     value: Cell<i32>,
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
///    const CLASS_NAME: &'static str = "MyClass";
///    type Kind = Native;
/// }
#[macro_export]
macro_rules! exports {
    [$export:expr, $($tt:tt)*] => {
        $crate::ExportList::new($export, exports!($($tt)*))
    };
    [$export:expr] => {
        $crate::ExportList::new($export, $crate::ExportNil)
    };
    [] => { $crate::ExportNil }
}

/// Define a list of methods to register with the game. Usually used in conjuction with
/// [`exports!`].
#[macro_export]
macro_rules! methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::method!($($mod)* $name, $ty::$id)),*] }
    };
}
