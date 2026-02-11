use std::marker::PhantomData;

use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
pub struct Curve<T>(red::CurveData, PhantomData<T>);

#[derive(Debug)]
#[repr(transparent)]
pub struct MultiChannelCurve<T>([u8; 56], PhantomData<T>);
