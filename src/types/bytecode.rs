use std::mem;

use super::{CName, Function, IScriptable, StackFrame};
use crate::VoidPtr;

pub const OPCODE_SIZE: isize = 1;
pub const CALL_INSTR_SIZE: isize = mem::size_of::<InvokeStatic>() as isize;

pub type OpcodeHandler = unsafe extern "C" fn(Option<&IScriptable>, &StackFrame, VoidPtr, VoidPtr);

pub trait Instr {
    const OPCODE: u8;
}

#[derive(Debug)]
#[repr(packed)]
pub struct InvokeStatic {
    pub skip: u16,
    pub line: u16,
    pub func: *mut Function,
    pub flags: u16,
}

impl Instr for InvokeStatic {
    const OPCODE: u8 = 36;
}

#[derive(Debug)]
#[repr(packed)]
pub struct InvokeVirtual {
    pub skip: u16,
    pub line: u16,
    pub name: CName,
    pub flags: u16,
}

impl Instr for InvokeVirtual {
    const OPCODE: u8 = 37;
}
