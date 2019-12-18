use conquer_pointer::{MarkedPtr, MaybeNull};

use crate::traits::Reclaimer;
use crate::typenum::Unsigned;
use crate::{Owned, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// StoreArg (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for abstracting over various possible argument types for store or
/// swap operations.
pub trait StoreArg {
    type Item: Sized;
    type Reclaimer: Reclaimer;
    type MarkBits: Unsigned;

    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;
}

/************ impl macros *************************************************************************/

/// Implements `StoreArg` for an unwrapped type.
macro_rules! impl_store_arg_for_type {
    () => {
        type Item = T;
        type Reclaimer = R;
        type MarkBits = N;

        #[inline]
        fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
            self.inner.into()
        }
    };
}

/// Implements `StoreArg` for `Option<_>`.
macro_rules! impl_store_arg_for_option {
    () => {
        type Item = T;
        type Reclaimer = R;
        type MarkBits = N;

        #[inline]
        fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
            match self {
                Some(ref ptr) => ptr.as_marked_ptr(),
                None => MarkedPtr::null(),
            }
        }
    };
}

/// Implements `StoreArg` for `MaybeNull<_>`.
macro_rules! impl_store_arg_for_maybe_null {
    () => {
        type Item = T;
        type Reclaimer = R;
        type MarkBits = N;

        #[inline]
        fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
            self.as_marked_ptr()
        }
    };
}

/********** Owned *********************************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Owned<T, R, N> {
    impl_store_arg_for_type!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Option<Owned<T, R, N>> {
    impl_store_arg_for_option!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for MaybeNull<Owned<T, R, N>> {
    impl_store_arg_for_maybe_null!();
}

/********** Shared ********************************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Shared<'_, T, R, N> {
    impl_store_arg_for_type!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Option<Shared<'_, T, R, N>> {
    impl_store_arg_for_option!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for MaybeNull<Shared<'_, T, R, N>> {
    impl_store_arg_for_maybe_null!();
}

/********** Unlinked ******************************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Unlinked<T, R, N> {
    impl_store_arg_for_type!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Option<Unlinked<T, R, N>> {
    impl_store_arg_for_option!();
}

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for MaybeNull<Unlinked<T, R, N>> {
    impl_store_arg_for_maybe_null!();
}

/********** Unprotected ***************************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> StoreArg for Unprotected<T, R, N> {
    impl_store_arg_for_type!();
}
