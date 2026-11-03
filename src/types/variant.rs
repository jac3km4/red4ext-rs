use std::{mem, ptr};

use crate::raw::root::RED4ext as red;
use crate::types::{CName, Type};
use crate::{FromRepr, IntoRepr, NativeRepr, RttiSystem};

/// Owns an RTTI-typed payload, stored inline or in engine-allocated storage.
/// Construction, cloning, and cleanup require initialized engine RTTI and allocators.
///
/// SDK: [Variant](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[repr(transparent)]
pub struct Variant(red::Variant);

impl Variant {
    /// Copies a converted representation into owned storage; returns `None` if
    /// its RTTI type is unavailable or SDK initialization fails.
    pub fn new<A: IntoRepr>(val: A) -> Option<Self> {
        let mut this = Self::default();
        let rtti = RttiSystem::get();
        let typ = rtti.get_type(CName::new(A::Repr::NAME))?;
        // Fill assigns the repr into separately owned storage; the source must still drop.
        let mut repr = val.into_repr();
        if !unsafe { this.0.Fill(typ.as_raw(), ptr::from_mut(&mut repr).cast()) } {
            return None;
        }
        Some(this)
    }

    /// Returns the payload's RTTI type, or `None` for an empty variant.
    #[inline]
    pub fn type_(&self) -> Option<&Type> {
        unsafe { Type::from_raw(self.0.GetType()) }
    }

    /// Borrows the payload's object representation, including uninitialized padding.
    ///
    /// Bytes are not guaranteed to be initialized, even when the payload is valid.
    /// Calling `assume_init` on any byte requires independent proof that that byte
    /// is initialized. This view is not a serialization format.
    /// Returns `None` if the variant has no type or payload storage.
    pub fn as_uninit_bytes(&self) -> Option<&[mem::MaybeUninit<u8>]> {
        let size = self.type_()?.size() as usize;
        let data_ptr = unsafe { self.0.GetDataPtr() }.cast::<mem::MaybeUninit<u8>>();
        if data_ptr.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(data_ptr, size) })
        }
    }

    /// Moves out a matching payload and empties this variant.
    /// Returns `None` without changing the variant if it is empty, its RTTI name
    /// does not match `A::Repr`, or its payload storage is unavailable.
    pub fn try_take<A: FromRepr>(&mut self) -> Option<A> {
        let typ = self.type_()?;
        if typ.name() != CName::new(A::Repr::NAME) {
            return None;
        }
        let repr = unsafe { self.0.GetDataPtr() }.cast::<A::Repr>();
        if repr.is_null() {
            return None;
        }
        Some(unsafe {
            self.take_repr::<A>(repr, |this, repr| {
                let typ = &*this.0.GetType().cast::<Type>();
                typ.allocator().free(repr);
            })
        })
    }

    // Requires a matching, initialized repr and a non-panicking, storage-only free callback.
    unsafe fn take_repr<A: FromRepr>(
        &mut self,
        repr: *mut A::Repr,
        free_storage: impl FnOnce(&Self, *mut A::Repr),
    ) -> A {
        let value = unsafe { ptr::read(repr) };
        // The payload has moved, so free only its backing storage, without Destruct.
        if !unsafe { self.0.IsInlined() } {
            free_storage(self, repr);
        }
        // Reset without Drop before conversion, which may panic.
        unsafe { ptr::write(self, Self::default()) }
        A::from_repr(value)
    }

    /// Borrows the payload if its RTTI name matches `A`.
    /// Returns `None` for an empty variant, a type mismatch, or missing storage.
    pub fn try_access<A: NativeRepr>(&self) -> Option<&A> {
        if self.type_()?.name() == CName::new(A::NAME) {
            unsafe { self.0.GetDataPtr().cast::<A>().as_ref() }
        } else {
            None
        }
    }
}

impl Default for Variant {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Clone for Variant {
    #[inline]
    fn clone(&self) -> Self {
        Self(unsafe { red::Variant::new4(&self.0) })
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe { self.0.Free() }
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::{Layout, dealloc};
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use super::*;

    #[repr(C)]
    struct Owned<const N: usize> {
        drops: Rc<Cell<usize>>,
        marker: [u64; N],
    }

    unsafe impl<const N: usize> NativeRepr for Owned<N> {
        const NAME: &'static str = "VariantTestOwned";
    }

    impl<const N: usize> Drop for Owned<N> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    struct PanickingConversion<const N: usize>;

    impl<const N: usize> FromRepr for PanickingConversion<N> {
        type Repr = Owned<N>;

        fn from_repr(_: Self::Repr) -> Self {
            panic!("synthetic conversion failure")
        }
    }

    fn check_owning_extraction<const N: usize>(inlined: bool, panic_conversion: bool) {
        let drops = Rc::new(Cell::new(0));
        let value = Owned {
            drops: Rc::clone(&drops),
            marker: [0x1234_5678_9abc_def0; N],
        };
        let typ = red::rtti::IType::default();
        let mut variant = Variant::default();
        // Only the SDK's storage/tag operations are used; no RTTI virtual calls.
        variant.0.type_ = ptr::from_ref(&typ).map_addr(|addr| addr | usize::from(inlined));
        let repr = if inlined {
            assert!(mem::size_of::<Owned<N>>() <= red::Variant_InlineSize as usize);
            assert!(mem::align_of::<Owned<N>>() <= red::Variant_InlineAlignment as usize);
            let repr = unsafe { variant.0.GetDataPtr() }.cast::<Owned<N>>();
            unsafe { ptr::write(repr, value) };
            repr
        } else {
            assert!(mem::size_of::<Owned<N>>() > red::Variant_InlineSize as usize);
            let repr = Box::into_raw(Box::new(value));
            variant.0.__bindgen_anon_1.instance = repr.cast();
            repr
        };
        let frees = Cell::new(0);
        let free_storage = |_: &Variant, memory: *mut Owned<N>| {
            assert!(!inlined, "inline storage must not be freed");
            assert_eq!(frees.get(), 0, "storage must be freed only once");
            assert_eq!(memory, repr);
            assert_eq!(drops.get(), 0);
            frees.set(frees.get() + 1);
            // Match the synthetic Box allocation, without dropping the moved payload.
            unsafe { dealloc(memory.cast(), Layout::new::<Owned<N>>()) };
        };

        let taken = if panic_conversion {
            let result = catch_unwind(AssertUnwindSafe(|| unsafe {
                variant.take_repr::<PanickingConversion<N>>(repr, free_storage)
            }));
            assert!(result.is_err());
            None
        } else {
            let taken = unsafe { variant.take_repr::<Owned<N>>(repr, free_storage) };
            assert_eq!(taken.marker, [0x1234_5678_9abc_def0; N]);
            assert_eq!(Rc::strong_count(&drops), 2);
            Some(taken)
        };

        assert_eq!(drops.get(), usize::from(panic_conversion));
        assert_eq!(frees.get(), usize::from(!inlined));
        assert!(variant.type_().is_none());
        assert!(unsafe { variant.0.GetDataPtr() }.is_null());
        assert!(variant.try_take::<Owned<N>>().is_none());
        drop(variant);
        assert_eq!(drops.get(), usize::from(panic_conversion));
        drop(taken);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn inline_extraction_moves_ownership_without_freeing_storage() {
        check_owning_extraction::<0>(true, false);
    }

    #[test]
    fn out_of_line_extraction_frees_only_storage() {
        check_owning_extraction::<2>(false, false);
    }

    #[test]
    fn extraction_resets_before_panicking_conversion() {
        check_owning_extraction::<0>(true, true);
        check_owning_extraction::<2>(false, true);
    }

    #[test]
    fn layout_matches_sdk() {
        assert_eq!(mem::size_of::<Variant>(), 0x18);
        assert_eq!(mem::align_of::<Variant>(), 8);
        assert_eq!(mem::size_of::<Variant>(), mem::size_of::<red::Variant>());
        assert_eq!(mem::align_of::<Variant>(), mem::align_of::<red::Variant>());
    }

    #[test]
    fn byte_view_exposes_maybe_uninitialized_storage() {
        // Pin the safe API to MaybeUninit: payload padding must not become &[u8].
        let variant = Variant::default();
        let bytes: Option<&[mem::MaybeUninit<u8>]> = variant.as_uninit_bytes();
        assert!(bytes.is_none());
    }

    #[test]
    fn taking_from_empty_variant_leaves_it_empty() {
        let mut variant = Variant::default();

        assert_eq!(variant.try_take::<u64>(), None);
        assert!(variant.type_().is_none());
        assert!(variant.as_uninit_bytes().is_none());
        assert!(variant.try_access::<u64>().is_none());
        assert_eq!(variant.try_take::<u64>(), None);
    }
}
