use conquer_pointer::MarkedPtr;
use typenum::Unsigned;

use crate::traits::Reclaimer;
use crate::{Owned, Shared, Unlinked, Unprotected};

pub trait StoreArg {
    type Item: Sized;
    type Reclaimer: Reclaimer;
    type MarkBits: Unsigned;

    fn as_marked_ptr(arg: &Self) -> MarkedPtr<T, N>;
    fn into_marked_ptr(arg: Self) -> MarkedPtr<T, N>;
}

impl<T, R: Reclaimer, N: Unsigned> StoreArg for Owned<T, R, N> {
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(arg: &Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Owned::as_marked_ptr(arg)
    }

    #[inline]
    fn into_marked_ptr(arg: Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Owned::into_marked_ptr(arg)
    }
}

impl<T, R: Reclaimer, N: Unsigned> StoreArg for Shared<'_, T, R, N> {
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(arg: &Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Shared::as_marked_ptr(arg)
    }

    #[inline]
    fn into_marked_ptr(arg: Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Shared::into_marked_ptr(arg)
    }
}

impl<T, R: Reclaimer, N: Unsigned> StoreArg for Unlinked<T, R, N> {
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(arg: &Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unimplemented!()
    }

    #[inline]
    fn into_marked_ptr(arg: Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unimplemented!()
    }
}

impl<T, R: Reclaimer, N: Unsigned> StoreArg for Unprotected<T, R, N> {
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(arg: &Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unimplemented!()
    }

    #[inline]
    fn into_marked_ptr(arg: Self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unimplemented!()
    }
}
