use std::marker::PhantomData;
use std::{iter, mem, ptr};

use super::{CName, Function, IScriptable, Instr, OPCODE_SIZE, Type, ValueContainer};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::{FromRepr, VoidPtr};

/// Borrowed script stack frame.
/// Reading arguments advances bytecode state; restore it before forwarding a hooked call.
///
/// SDK: [CStackFrame](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Stack.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct StackFrame(red::CStackFrame);

impl StackFrame {
    /// Borrows the current function metadata.
    ///
    /// Use only on a frame with valid function metadata, such as a scripted frame;
    /// the native pointer is not checked for null.
    #[inline]
    pub fn func(&self) -> &Function {
        unsafe { &*(self.0.func as *const Function) }
    }

    /// Returns the parent stack frame.
    #[inline]
    pub fn parent(&self) -> Option<&StackFrame> {
        unsafe { (self.0.parent as *const StackFrame).as_ref() }
    }

    /// Iterates from the immediate parent outward through ancestor frames.
    #[inline]
    pub fn parent_iter(&self) -> impl Iterator<Item = &StackFrame> {
        iter::successors(self.parent(), |frame| frame.parent())
    }

    /// Returns the context of the stack frame, the `this` pointer.
    #[inline]
    pub fn context(&self) -> Option<&IScriptable> {
        unsafe { (self.0.context as *const IScriptable).as_ref() }
    }

    /// Returns whether the current bytecode pointer is non-null.
    #[inline]
    pub fn has_code(&self) -> bool {
        !self.0.code.is_null()
    }

    /// Returns a view of scripted-frame local storage, which may have a null address.
    #[inline]
    pub fn locals(&self) -> ValueContainer {
        ValueContainer::new(self.0.localVars)
    }

    /// Returns a view of scripted-frame parameter storage, which may have a null address.
    #[inline]
    pub fn params(&self) -> ValueContainer {
        ValueContainer::new(self.0.params)
    }

    /// Reads operands of `I` at a byte offset from the current bytecode pointer.
    ///
    /// Returns `None` if there is no bytecode or the opcode does not match `I`.
    ///
    /// # Safety
    /// The offset must point to a readable opcode within the current bytecode
    /// allocation. If it matches, the following bytes must be valid operands of
    /// `I` and remain valid and unchanged for the returned borrow.
    pub unsafe fn instr_at<I: Instr>(&self, offset: isize) -> Option<&I> {
        if self.0.code.is_null() {
            return None;
        }
        unsafe {
            let ptr = self.0.code.offset(offset);
            (ptr.read() as u8 == I::OPCODE).then(|| &*(ptr.offset(OPCODE_SIZE) as *const I))
        }
    }

    /// Advances the bytecode pointer by one opcode byte, not a whole instruction.
    ///
    /// # Safety
    /// The current pointer and the advanced pointer must lie within the same
    /// valid bytecode allocation (or one byte past its end).
    #[inline]
    pub unsafe fn step(&mut self) {
        self.0.code = unsafe { self.0.code.offset(OPCODE_SIZE) };
    }

    /// Evaluates and consumes the next argument, advancing the frame's bytecode state.
    ///
    /// # Safety
    /// The frame must point to a valid next argument expression, and `T` must
    /// match its type. Any engine systems needed to evaluate it must be initialized.
    /// Restore the argument state before forwarding a hooked call.
    #[inline]
    pub unsafe fn get_arg<T>(&mut self) -> T
    where
        T: FromRepr,
    {
        let mut repr = mem::MaybeUninit::zeroed();
        unsafe { self.read_arg(ptr::from_mut(&mut repr).cast()) };
        T::from_repr(unsafe { repr.assume_init() })
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

    /// Captures bytecode and argument-data pointers for [`restore_args`](Self::restore_args).
    ///
    /// Capture before reading any arguments; the argument index is not saved.
    pub fn args_state(&self) -> StackArgsState {
        StackArgsState {
            code: self.0.code,
            data: self.0.data,
            data_type: self.0.dataType,
        }
    }

    /// Restores saved argument pointers and resets the argument index to zero.
    ///
    /// This does not undo side effects of evaluating argument expressions.
    ///
    /// # Safety
    /// The snapshot must come from this frame before any arguments were read,
    /// and its pointers must still be valid. Read the full argument list and
    /// restore it before forwarding the frame to the original game function.
    ///
    /// # Example
    /// ```rust
    /// # use red4ext_rs::types::{CName, StackFrame};
    /// // For a hooked function taking exactly one CName argument:
    /// unsafe fn inspect(frame: &mut StackFrame) {
    ///     let state = frame.args_state();
    ///     let name: CName = unsafe { frame.get_arg() };
    ///     // Inspect `name`, then restore before forwarding the call.
    ///     unsafe { frame.restore_args(state) };
    /// }
    /// ```
    pub unsafe fn restore_args(&mut self, state: StackArgsState) {
        self.0.code = state.code;
        self.0.data = state.data;
        self.0.dataType = state.data_type;
        self.0.currentParam = 0;
    }
}

/// Borrowed argument descriptor for a native stack call.
///
/// SDK: [CStack](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Stack.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct StackArg<'a>(red::CStackType, PhantomData<&'a mut ()>);

impl<'a> StackArg<'a> {
    /// Borrows a value as a call argument, resolving its native type through RTTI.
    ///
    /// Returns `None` if the type is unregistered. `Void` uses a null type pointer
    /// without a lookup; other types require initialized engine RTTI.
    pub fn new<A: NativeRepr>(val: &'a mut A) -> Option<Self> {
        let type_ = if A::NAME == "Void" {
            ptr::null_mut()
        } else {
            let rtti = RttiSystem::get();
            rtti.get_type(CName::new(A::NAME))?.as_raw() as *const _ as *mut red::rtti::IType
        };
        let inner = red::CStackType {
            type_,
            value: val as *const A as VoidPtr,
        };
        Some(Self(inner, PhantomData))
    }

    /// Returns the argument's RTTI type, or `None` for `Void`.
    #[inline]
    pub fn type_(&self) -> Option<&Type> {
        unsafe { self.0.type_.cast::<Type>().as_ref() }
    }

    #[inline]
    pub(super) fn as_raw_mut(&mut self) -> &mut red::CStackType {
        &mut self.0
    }
}

/// Argument pointers captured by [`StackFrame::args_state`].
///
/// Does not own the referenced data or save the argument index. Restore only to
/// the originating live frame using [`StackFrame::restore_args`].
pub struct StackArgsState {
    code: *mut i8,
    data: VoidPtr,
    data_type: *mut red::rtti::IType,
}
