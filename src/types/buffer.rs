use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
/// SDK [DataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp) layout wrapper.
pub struct DataBuffer(red::DataBuffer);

#[derive(Debug)]
#[repr(transparent)]
/// SDK [DeferredDataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp) layout wrapper.
pub struct DeferredDataBuffer(red::DeferredDataBuffer);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
/// SDK [SharedDataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp) layout wrapper.
pub struct SharedDataBuffer(red::SharedDataBuffer);
