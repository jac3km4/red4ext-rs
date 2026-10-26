use std::mem;

use sealed::sealed;

use super::{CName, Function, IScriptable, StackFrame};
use crate::VoidPtr;

pub const OPCODE_SIZE: isize = 1;
pub const CALL_INSTR_SIZE: isize = mem::size_of::<InvokeStatic>() as isize;

/// SDK [opcode handler](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/OpcodeHandlers.hpp) signature.
pub type OpcodeHandler = unsafe extern "C" fn(*mut IScriptable, *mut StackFrame, VoidPtr, VoidPtr);

/// A trait for types that correspond to bytecode instructions.
#[sealed]
pub trait Instr {
    const OPCODE: u8;
}

#[derive(Debug)]
#[repr(C, packed)]
/// Packed operands of the static-call opcode, excluding the opcode byte.
pub struct InvokeStatic {
    pub skip: u16,
    pub line: u16,
    pub func: *mut Function,
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
    pub skip: u16,
    pub line: u16,
    pub name: CName,
    pub flags: u16,
}

#[sealed]
impl Instr for InvokeVirtual {
    const OPCODE: u8 = 37;
}
