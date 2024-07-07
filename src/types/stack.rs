use std::marker::PhantomData;
use std::{iter, ptr};

use super::{CName, Function, IScriptable, Instr, Type, ValueContainer, OPCODE_SIZE};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::VoidPtr;

#[derive(Debug)]
#[repr(transparent)]
pub struct StackFrame(red::CStackFrame);

impl StackFrame {
    #[inline]
    pub fn func(&self) -> &Function {
        unsafe { &*(self.0.func as *const Function) }
    }

    #[inline]
    pub fn parent(&self) -> Option<&StackFrame> {
        unsafe { (self.0.parent as *const StackFrame).as_ref() }
    }

    #[inline]
    pub fn parent_iter(&self) -> impl Iterator<Item = &StackFrame> {
        iter::successors(self.parent(), |frame| frame.parent())
    }

    #[inline]
    pub fn context(&self) -> Option<&IScriptable> {
        unsafe { (self.0.context as *const IScriptable).as_ref() }
    }

    #[inline]
    pub fn has_code(&self) -> bool {
        !self.0.code.is_null()
    }

    #[inline]
    pub fn locals(&self) -> ValueContainer {
        ValueContainer::new(self.0.localVars)
    }

    #[inline]
    pub fn params(&self) -> ValueContainer {
        ValueContainer::new(self.0.params)
    }

    pub unsafe fn instr_at<I: Instr>(&self, offset: isize) -> Option<&I> {
        if self.0.code.is_null() {
            return None;
        }
        let ptr = self.0.code.offset(offset);
        (ptr.read() as u8 == I::OPCODE).then(|| &*(ptr.offset(OPCODE_SIZE) as *const I))
    }

    #[inline]
    pub unsafe fn step(&mut self) {
        self.0.code = unsafe { self.0.code.offset(OPCODE_SIZE) };
    }

    #[inline]
    pub unsafe fn get_arg<T: Default>(&mut self) -> T {
        let mut out = T::default();
        self.read_arg(&mut out as *mut T as VoidPtr);
        out
    }

    unsafe fn read_arg(&mut self, ptr: VoidPtr) {
        self.0.data = ptr::null_mut();
        self.0.dataType = ptr::null_mut();
        self.0.currentParam += 1;
        unsafe {
            let opcode = *self.0.code as u8;
            self.step();
            red::OpcodeHandlers::Run(opcode, self.0.context, &mut self.0, ptr, ptr::null_mut());
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct StackArg<'a>(red::CStackType, PhantomData<&'a ()>);

impl<'a> StackArg<'a> {
    pub fn new<A: NativeRepr>(val: &'a mut A) -> Option<Self> {
        let rtti = RttiSystem::get();
        let type_ = rtti.get_type(CName::new(A::NATIVE_NAME))?;
        let inner = red::CStackType {
            type_: type_.as_raw() as *const _ as *mut red::CBaseRTTIType,
            value: val as *const A as VoidPtr,
        };
        Some(Self(inner, PhantomData))
    }

    #[inline]
    pub fn type_(&self) -> Option<&'static Type> {
        unsafe { self.0.type_.cast::<Type>().as_ref() }
    }

    #[inline]
    pub(super) fn as_raw_mut(&mut self) -> &mut red::CStackType {
        &mut self.0
    }
}
