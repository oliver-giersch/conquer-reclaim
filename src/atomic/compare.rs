use core::marker::PhantomData;

use conquer_pointer::MaybeNull;

use crate::atomic::store::StoreArg;
use crate::typenum::Unsigned;
use crate::{Reclaim, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareArg (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for abstracting over various possible argument types for the compare
/// part of a CAS.
pub trait CompareArg: StoreArg {
    type Unlinked: Sized;

    unsafe fn into_unlinked(self) -> Self::Unlinked;
}

/// Implements `CompareArg` for an unwrapped type.
macro_rules! impl_compare_arg_for_type {
    () => {
        type Unlinked = Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>;

        #[inline]
        unsafe fn into_unlinked(self) -> Self::Unlinked {
            Unlinked { inner: self.inner, _marker: PhantomData }
        }
    };
}

/// Implements `CompareArg` for `Option<T>`.
macro_rules! impl_compare_arg_for_option {
    () => {
        type Unlinked = Option<Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>>;

        #[inline]
        unsafe fn into_unlinked(self) -> Self::Unlinked {
            self.map(|ptr| Unlinked { inner: ptr.inner, _marker: PhantomData })
        }
    };
}

/// Implements `CompareArg` for `MaybeNull<T>`.
macro_rules! impl_compare_arg_for_maybe_null {
    () => {
        type Unlinked = MaybeNull<Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>>;

        #[inline]
        unsafe fn into_unlinked(self) -> Self::Unlinked {
            self.map(|ptr| Unlinked { inner: ptr.inner, _marker: PhantomData })
        }
    };
}

/********** Shared ********************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for Shared<'_, T, R, N> {
    impl_compare_arg_for_type!();
}

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for Option<Shared<'_, T, R, N>> {
    impl_compare_arg_for_option!();
}

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for MaybeNull<Shared<'_, T, R, N>> {
    impl_compare_arg_for_maybe_null!();
}

/********** Unlinked ******************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for Unlinked<T, R, N> {
    impl_compare_arg_for_type!();
}

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for Option<Unlinked<T, R, N>> {
    impl_compare_arg_for_option!();
}

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for MaybeNull<Unlinked<T, R, N>> {
    impl_compare_arg_for_maybe_null!();
}

/********** Unprotected ***************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> CompareArg for Unprotected<T, R, N> {
    type Unlinked = MaybeNull<Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    #[inline]
    unsafe fn into_unlinked(self) -> Self::Unlinked {
        MaybeNull::from(self.as_marked_ptr()).map(|inner| Unlinked { inner, _marker: PhantomData })
    }
}
