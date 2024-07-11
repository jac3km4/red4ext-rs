use sealed::sealed;

use crate::invocable::{GlobalMetadata, MethodMetadata};
use crate::systems::RttiSystemMut;
use crate::types::{CName, NativeClass, ScriptClass};

#[derive(Debug)]
pub struct ExportList<H, T> {
    head: H,
    tail: T,
}

impl<H, T> ExportList<H, T> {
    pub const fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

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

#[derive(Debug)]
pub struct ExportNil;

#[sealed]
impl Exportable for ExportNil {
    #[inline]
    fn register(&self) {}

    #[inline]
    fn post_register(&self) {}
}

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

#[derive(Debug)]
pub struct ClassExportBuilder<C: 'static> {
    base: Option<&'static str>,
    methods: &'static [MethodMetadata<C>],
}

impl<C> ClassExportBuilder<C> {
    pub const fn base(mut self, base: &'static str) -> Self {
        self.base = Some(base);
        self
    }

    pub const fn methods(mut self, methods: &'static [MethodMetadata<C>]) -> Self {
        self.methods = methods;
        self
    }

    pub const fn build(self) -> ClassExport<C> {
        ClassExport {
            base: self.base,
            methods: self.methods,
        }
    }
}

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

#[macro_export]
macro_rules! methods {
    [$( $($mod:ident)* $name:literal => $ty:ident::$id:ident),*$(,)?] => {
        const { &[$($crate::method!($($mod)* $name, $ty::$id)),*] }
    };
}
