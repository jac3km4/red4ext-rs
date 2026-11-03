use std::mem;

use sealed::sealed;

use super::{CName, Function, IScriptable, StackFrame};
use crate::VoidPtr;

/// Size in bytes of an instruction's opcode, excluding its operands.
pub const OPCODE_SIZE: isize = 1;
/// Size in bytes of static-call operands, excluding the opcode byte.
pub const CALL_INSTR_SIZE: isize = mem::size_of::<InvokeStatic>() as isize;

/// Script opcode handler.
///
/// Arguments are the script context, stack frame, result storage, and an
/// opcode-specific auxiliary pointer. Callers must meet the opcode's requirements.
///
/// SDK: [opcode handler](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/OpcodeHandlers.hpp).
pub type OpcodeHandler = unsafe extern "C" fn(*mut IScriptable, *mut StackFrame, VoidPtr, VoidPtr);

/// Packed operand layouts recognized by [`StackFrame::instr_at`].
#[sealed]
pub trait Instr {
    /// Opcode byte that precedes these operands.
    const OPCODE: u8;
}

#[derive(Debug)]
#[repr(C, packed)]
/// Packed operands of the static-call opcode, excluding the opcode byte.
pub struct InvokeStatic {
    /// Encoded skip offset.
    pub skip: u16,
    /// Source line number.
    pub line: u16,
    /// Target function metadata; this pointer does not convey ownership.
    pub func: *mut Function,
    /// Encoded call flags.
    pub flags: u16,
}

#[sealed]
impl Instr for InvokeStatic {
    const OPCODE: u8 = 36;
}

#[derive(Debug)]
#[repr(C, packed)]
/// Packed operands of the virtual-call opcode, excluding the opcode byte.
pub struct InvokeVirtual {
    /// Encoded skip offset.
    pub skip: u16,
    /// Source line number.
    pub line: u16,
    /// Method name used for virtual dispatch.
    pub name: CName,
    /// Encoded call flags.
    pub flags: u16,
}

#[sealed]
impl Instr for InvokeVirtual {
    const OPCODE: u8 = 37;
}
