use std::marker::PhantomData;

use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
/// Typed layout wrapper for SDK
/// [CurveData](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
pub struct Curve<T>(red::CurveData, PhantomData<T>);

#[derive(Debug)]
#[repr(transparent)]
/// Opaque multichannel curve layout. `T` is a marker, not an accessible element buffer.
pub struct MultiChannelCurve<T>([u8; 56], PhantomData<T>);
