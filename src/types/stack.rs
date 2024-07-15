use std::marker::PhantomData;
use std::{iter, ptr};

use super::{CName, Function, IScriptable, Instr, Type, ValueContainer, OPCODE_SIZE};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::VoidPtr;

/// A script stack frame.
#[derive(Debug)]
#[repr(transparent)]
pub struct StackFrame(red::CStackFrame);

impl StackFrame {
    /// Returns the current function of the stack frame.
    #[inline]
    pub fn func(&self) -> &Function {
        unsafe { &*(self.0.func as *const Function) }
    }

    /// Returns the parent stack frame.
    #[inline]
    pub fn parent(&self) -> Option<&StackFrame> {
        unsafe { (self.0.parent as *const StackFrame).as_ref() }
    }

    /// Returns an iterator over all parent stack frames.
    #[inline]
    pub fn parent_iter(&self) -> impl Iterator<Item = &StackFrame> {
        iter::successors(self.parent(), |frame| frame.parent())
    }

    /// Returns the context of the stack frame, the `this` pointer.
    #[inline]
    pub fn context(&self) -> Option<&IScriptable> {
        unsafe { (self.0.context as *const IScriptable).as_ref() }
    }

    /// Returns `true` if the stack frame has a code block.
    #[inline]
    pub fn has_code(&self) -> bool {
        !self.0.code.is_null()
    }

    /// Returns the memory address where local variables are stored.
    #[inline]
    pub fn locals(&self) -> ValueContainer {
        ValueContainer::new(self.0.localVars)
    }

    /// Returns the memory address where parameters are stored.
    #[inline]
    pub fn params(&self) -> ValueContainer {
        ValueContainer::new(self.0.params)
    }

    /// Interprets the code at specified offset as an instruction of type `I`.
    pub unsafe fn instr_at<I: Instr>(&self, offset: isize) -> Option<&I> {
        if self.0.code.is_null() {
            return None;
        }
        let ptr = self.0.code.offset(offset);
        (ptr.read() as u8 == I::OPCODE).then(|| &*(ptr.offset(OPCODE_SIZE) as *const I))
    }

    /// Steps over a single opcode (1 byte).
    #[inline]
    pub unsafe fn step(&mut self) {
        self.0.code = unsafe { self.0.code.offset(OPCODE_SIZE) };
    }

    /// Retrieves the next argument from the stack frame.
    ///
    /// # Safety
    /// The type `T` must be the correct type of the next argument.
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

    pub fn state(&self) -> StackState {
        StackState {
            code: self.0.code,
            data: self.0.data,
            data_type: self.0.dataType,
        }
    }

    /// Allows to rewind the stack after having read function arguments,
    /// when detoured function need to be called.
    ///
    /// # Safety
    /// The state must be saved **before** reading arguments.
    /// The state must be restored **after** having read arguments, before calling callback.
    ///
    /// # Example
    /// ```rust
    /// # use red4ext_rs::{hooks, SdkEnv, types::{CName, StackFrame, IScriptable}, VoidPtr};
    /// # hooks! {
    /// #    static ADD_HOOK: fn(a: u32, b: u32) -> u32;
    /// # }
    /// # fn attach_my_hook(env: &SdkEnv, addr: unsafe extern "C" fn(i: *mut IScriptable, f: *mut StackFrame, a3: VoidPtr, a4: VoidPtr)) {
    /// #     unsafe { env.attach_hook(ADD_HOOK, addr, detour) };
    /// # }
    /// # fn should_detour(event_name: CName) -> bool = false;
    ///
    /// unsafe extern "C" fn detour(
    ///     i: *mut IScriptable,
    ///     f: *mut StackFrame,
    ///     a3: VoidPtr,
    ///     a4: VoidPtr,
    ///     cb: unsafe extern "C" fn(i: *mut IScriptable, f: *mut StackFrame, a3: VoidPtr, a4: VoidPtr),
    /// ) {
    ///     let frame = &mut *f;
    ///
    ///     // stack must be saved before reading stack function parameters
    ///     let state = frame.state();
    ///     
    ///     // assuming our function accepts these 3 parameters
    ///     let event_name: CName = StackFrame::get_arg(frame);
    ///     let entity_id: EntityId = StackFrame::get_arg(frame);
    ///     let emitter_name: CName = StackFrame::get_arg(frame);
    ///
    ///     if should_detour(event_name) {
    ///         // do something else...
    ///     } else {
    ///         // since we've read stack function arguments,
    ///         // stack must be rewinded before callback.
    ///         frame.rewind(state);
    ///         cb(a, b)
    ///     }
    /// }
    /// ```
    pub unsafe fn rewind(&mut self, state: StackState) {
        self.0.code = state.code;
        self.0.data = state.data;
        self.0.dataType = state.data_type;
        self.0.currentParam = 0;
    }
}

/// A stack argument to be passed to a function.
#[derive(Debug)]
#[repr(transparent)]
pub struct StackArg<'a>(red::CStackType, PhantomData<&'a mut ()>);

impl<'a> StackArg<'a> {
    /// Creates a new stack argument from a reference to a value.
    pub fn new<A: NativeRepr>(val: &'a mut A) -> Option<Self> {
        let type_ = if A::NAME == "Void" {
            ptr::null_mut()
        } else {
            let rtti = RttiSystem::get();
            rtti.get_type(CName::new(A::NAME))?.as_raw() as *const _ as *mut red::CBaseRTTIType
        };
        let inner = red::CStackType {
            type_,
            value: val as *const A as VoidPtr,
        };
        Some(Self(inner, PhantomData))
    }

    /// Returns the type of the stack argument.
    #[inline]
    pub fn type_(&self) -> Option<&Type> {
        unsafe { self.0.type_.cast::<Type>().as_ref() }
    }

    #[inline]
    pub(super) fn as_raw_mut(&mut self) -> &mut red::CStackType {
        &mut self.0
    }
}

pub struct StackState {
    code: *mut i8,
    data: VoidPtr,
    data_type: *mut red::CBaseRTTIType,
}
