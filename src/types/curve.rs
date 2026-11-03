use std::marker::PhantomData;

use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
/// Opaque typed curve data.
///
/// `T` identifies the curve's value type; point access and evaluation are not exposed.
///
/// SDK: [CurveData](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
pub struct Curve<T>(red::CurveData, PhantomData<T>);

#[derive(Debug)]
#[repr(transparent)]
/// Opaque multichannel curve value.
///
/// `T` identifies the value type; channel access and evaluation are not exposed.
pub struct MultiChannelCurve<T>([u8; 56], PhantomData<T>);
