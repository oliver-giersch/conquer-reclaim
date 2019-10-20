use core::convert::TryFrom;
use core::mem;
use core::ptr;

use conquer_pointer::{
    MarkedNonNull, MarkedNonNullable,
    MarkedOption::{self, Null, Value},
    MarkedPtr,
};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::{Reclaim, SharedPointer};

/********** blanket impl for Option ***************************************************************/

impl<P, T, R: Reclaim, N: Unsigned> SharedPointer for Option<P>
where
    P: SharedPointer<Pointer = P, Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
        + Internal,
{
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;
    type Pointer = P;

    #[inline]
    fn with(ptr: Self::Pointer) -> Self {
        Some(ptr)
    }

    #[inline]
    fn compose(ptr: Self::Pointer, tag: usize) -> Self {
        Some(ptr.with_tag(tag))
    }

    #[inline]
    unsafe fn from_marked_ptr(marked_ptr: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        match MarkedNonNull::try_from(marked_ptr) {
            Ok(ptr) => Some(Self::Pointer::from_marked_non_null(ptr)),
            Err(_) => None,
        }
    }

    #[inline]
    unsafe fn from_marked_non_null(marked_ptr: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Some(Self::Pointer::from_marked_non_null(marked_ptr))
    }

    #[inline]
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Some(ptr) => ptr.as_marked_ptr(),
            None => MarkedPtr::null(),
        }
    }

    #[inline]
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Some(ptr) => ptr.into_marked_ptr(),
            None => MarkedPtr::null(),
        }
    }

    #[inline]
    fn clear_tag(self) -> Self {
        self.map(|ptr| ptr.clear_tag())
    }

    #[inline]
    fn with_tag(self, tag: usize) -> Self {
        self.map(|ptr| ptr.with_tag(tag))
    }

    #[inline]
    fn decompose(self) -> (Self, usize) {
        match self {
            Some(ptr) => {
                let (ptr, tag) = ptr.decompose();
                (Some(ptr), tag)
            }
            None => (None, 0),
        }
    }
}

/********** blanket impl for MarkedOption *********************************************************/

impl<P, T, R: Reclaim, N: Unsigned> SharedPointer for MarkedOption<P>
where
    P: SharedPointer<Pointer = P, Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
        + Internal,
{
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;
    type Pointer = P;

    #[inline]
    fn with(ptr: Self::Pointer) -> Self {
        Value(ptr)
    }

    #[inline]
    fn compose(ptr: Self::Pointer, tag: usize) -> Self {
        Value(ptr.with_tag(tag))
    }

    #[inline]
    unsafe fn from_marked_ptr(marked_ptr: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        MarkedOption::from(marked_ptr).map(|ptr| Self::Pointer::from_marked_non_null(ptr))
    }

    #[inline]
    unsafe fn from_marked_non_null(marked_ptr: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Value(Self::Pointer::from_marked_non_null(marked_ptr))
    }

    #[inline]
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Value(ptr) => ptr.as_marked_ptr(),
            Null(tag) => MarkedPtr::compose(ptr::null_mut(), *tag),
        }
    }

    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        let ptr = self.as_marked_ptr();
        mem::forget(self);
        ptr
    }

    #[inline]
    fn clear_tag(self) -> Self {
        match self {
            Value(ptr) => Value(ptr.clear_tag()),
            Null(_) => Null(0),
        }
    }

    #[inline]
    fn with_tag(self, tag: usize) -> Self {
        match self {
            Value(ptr) => Value(ptr.with_tag(tag)),
            Null(_) => Null(tag),
        }
    }

    #[inline]
    fn decompose(self) -> (Self, usize) {
        match self {
            Value(ptr) => {
                let (ptr, tag) = ptr.decompose();
                (Value(ptr), tag)
            }
            Null(tag) => (Null(0), 0),
        }
    }
}
