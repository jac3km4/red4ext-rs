use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
pub struct DataBuffer(red::DataBuffer);

#[derive(Debug)]
#[repr(transparent)]
pub struct DeferredDataBuffer(red::DeferredDataBuffer);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct SharedDataBuffer(red::SharedDataBuffer);
