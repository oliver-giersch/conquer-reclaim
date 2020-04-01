use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::traits::Reclaim;
use crate::Shared;

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Shared<'_, T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Shared<'_, T, R, N> {}

/********** impl inherent (const) *****************************************************************/

impl<T, R, N> Shared<'_, T, R, N> {
    pub const unsafe fn cast<'a, U>(self) -> Shared<'a, U, R, N> {
        Shared { inner: self.inner.cast(), _marker: PhantomData }
    }
}

/********** impl inherent *************************************************************************/

impl<'g, T, R: Reclaim, N: Unsigned> Shared<'g, T, R, N> {
    impl_from_ptr!();
    impl_from_non_null!();
    impl_common!();

    /// De-references the [`Shared`] reference.
    ///
    /// # Safety
    ///
    /// Since [`Shared`]s can not be `null` and can only be (safely) created
    /// by protecting their referenced value from concurrent reclamation,
    /// de-referencing them is generally sound.
    /// However, it does require the caller to ensure correct synchronization
    /// and especially memory orderings are in place.
    /// Consider the following example:
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use conquer_reclaim::prelude::*;
    /// use conquer_reclaim::leak::{Atomic, Guard, Owned, Shared};
    /// use conquer_reclaim::typenum::U0;
    ///
    /// #[derive(Default)]
    /// struct Foo {
    ///     bar: i32,
    /// }
    ///
    /// static GLOBAL: Atomic<Foo, U0> = Atomic::null();
    ///
    /// // ...in thread 1
    /// let mut owned = Owned::new(Default::default());
    /// owned.bar = -1;
    ///
    /// GLOBAL.store(owned, Ordering::Release);
    ///
    /// // ...in thread 2
    /// if let NotNull(shared) = GLOBAL.load(&mut &Guard, Ordering::Acquire) {
    ///     let foo = unsafe { shared.deref() };
    ///     assert_eq!(foo.bar, -1);
    /// }
    /// ```
    ///
    /// Due to the correct pairing of [`Acquire`][acq] and [`Release`][rel] in
    /// this example calling `deref` in thread 2 is sound.
    /// If, however, both memory orderings were set to [`Relaxed`][rlx], this
    /// would not be case, since the compiler/cpu would be allowed to re-order
    /// the write `owned.bar = -1` after the write (store) to `GLOBAL`, in which
    /// case there could be data-race when reading `foo.bar` in thread 2.
    /// Calling `deref` is **always** sound when [`SeqCst`][scs] is used
    /// exclusively.
    ///
    /// [acq]: core::sync::atomic::Ordering::Acquire
    /// [rel]: core::sync::atomic::Ordering::Release
    /// [rlx]: core::sync::atomic::Ordering::Relaxed
    /// [scs]: core::sync::atomic::Ordering::SeqCst
    #[inline]
    pub unsafe fn as_ref(self) -> &'g T {
        &*self.inner.decompose_ptr()
    }

    /// Decomposes and de-references the [`Shared`] reference and returns both
    /// the reference and its tag value.
    ///
    /// # Safety
    ///
    /// See [`deref`][Shared::deref] for an explanation of the safety concerns
    /// involved in de-referencing a [`Shared`].
    #[inline]
    pub unsafe fn decompose_ref(self) -> (&'g T, usize) {
        let (ptr, tag) = self.inner.decompose();
        (&*ptr.as_ptr(), tag)
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug, R, N: Unsigned + 'static> fmt::Debug for Shared<'_, T, R, N> {
    impl_fmt_debug!(Shared);
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> fmt::Pointer for Shared<'_, T, R, N> {
    impl_fmt_pointer!();
}
