use core::mem;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper type that prepends an instance of `T` with an instance of the
/// [`Header`][Reclaim::Header] type associated to the specified
/// [`Reclaimer`][Reclaim].
///
/// This struct guarantees the layout of its fields to match the declaration
/// order, i.e., the `header` always precedes the `data`.
#[repr(C)]
pub(crate) struct Record<H, T: ?Sized> {
    /// The record's header
    pub header: H,
    /// The wrapped record data itself.
    pub data: T,
}

/********** impl inherent *************************************************************************/

impl<H, T: ?Sized> Record<H, T> {
    /// [`Record`] is marked as `#[repr(c)]` and hence the offset of the header
    /// field is always `0`.
    const HEADER_OFFSET: usize = 0;

    #[inline]
    pub unsafe fn record_from_data(data: *mut T) -> *mut Self {
        // pointer is cast to a pointer to an unsized `Record<H, dyn ..>`, but
        // in fact still points at the record's `data` field
        let mut ptr = data as *mut Self;
        // header is the "correct" thin pointer, since the header field is at
        // offset 0
        let header = Self::header_from_data(data) as *mut ();
        {
            // create a pointer to the fat pointer's "data" half
            // this is the dangerous part, since the layout of fat pointers to
            // trait objects is not actually guaranteed in any way, but is
            // currently implemented this way
            let data_ptr = &mut ptr as *mut *mut Self as *mut *mut ();
            *data_ptr = header;
        }

        ptr
    }

    /// Returns the pointer to the [`header`][Record::header] field of the
    /// [`Record`] containing the value pointed to by `data`.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid, un-aliased pointer to an instance of
    /// `T` that was constructed as part of a [`Record`].
    #[inline]
    pub unsafe fn header_from_data(data: *const T) -> *mut H {
        // TODO: use align_of_val_raw once it becomes stable
        let data_align = mem::align_of_val(&*data);
        (data as *mut u8).sub(Self::data_offset(data_align)).cast()
    }

    /// Returns the offset in bytes from the [`Record`] to its `data` field for
    /// a given `data_align` of that field's type (of which the concrete type
    /// may not be known).
    #[inline]
    const fn data_offset(data_align: usize) -> usize {
        // this matches the layout algorithm used by `rustc` for C-like structs.
        let offset = Self::HEADER_OFFSET + mem::size_of::<H>();
        offset + offset.wrapping_neg() % data_align
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type Record<T> = super::Record<usize, T>;

    trait TypeName {
        fn type_name(&self) -> &'static str;
    }

    struct DropCounting<'a>(&'a AtomicUsize);

    impl Drop for DropCounting<'_> {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl TypeName for DropCounting<'_> {
        fn type_name(&self) -> &'static str {
            "DropCounting"
        }
    }

    impl TypeName for [u8; 128] {
        fn type_name(&self) -> &'static str {
            "[u8; 128]"
        }
    }

    #[test]
    fn dyn_trait_record() {
        const MAGIC1: usize = 0xDEAD_BEEF;
        const MAGIC2: usize = 0xCAFE_BABE;

        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let counting = Box::leak(Box::new(Record { header: MAGIC1, data: DropCounting(&COUNT) }))
            as &mut Record<dyn TypeName>;
        let counting = &mut counting.data as *mut dyn TypeName;
        let array = Box::leak(Box::new(Record { header: MAGIC2, data: [0u8; 128] }))
            as &mut Record<dyn TypeName>;
        let array = &mut array.data as *mut dyn TypeName;

        unsafe {
            let counting = Box::from_raw(Record::record_from_data(counting));
            assert_eq!(counting.header, MAGIC1);
            assert_eq!(counting.data.type_name(), "DropCounting");
            assert_eq!(mem::size_of_val(&counting.data), mem::size_of::<DropCounting<'_>>());

            let array = Box::from_raw(Record::record_from_data(array));
            assert_eq!(array.header, MAGIC2);
            assert_eq!(array.data.type_name(), "[u8; 128]");
            assert_eq!(mem::size_of_val(&array.data), mem::size_of::<[u8; 128]>());
        }

        assert_eq!(COUNT.load(Ordering::Relaxed), 1);
    }
}
