use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::conquer_pointer::MaybeNull::NotNull;
use conquer_reclaim::{GenericReclaimer, GlobalReclaimer, Owned, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, conquer_reclaim::typenum::U0>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

// FIXME: pad head & tail to 128 byte
pub struct Queue<T, R: GenericReclaimer> {
    head: Atomic<Node<T, R>, R>,
    tail: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: GenericReclaimer> Queue<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);
    const RELAXED_CAS: (Ordering, Ordering) = (Ordering::Relaxed, Ordering::Relaxed);

    /// Creates a new empty [`Queue`].
    #[inline]
    pub fn new() -> Self {
        let sentinel = Owned::leak_unprotected(Owned::new(Node::sentinel()));
        Self {
            head: unsafe { Atomic::from_unprotected(sentinel) },
            tail: unsafe { Atomic::from_unprotected(sentinel) },
            reclaimer: R::new(),
        }
    }

    /// Returns `true` if the [`Queue`] is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load_raw(Ordering::Relaxed) == self.tail.load_raw(Ordering::Relaxed)
    }

    /// Derives a [`ReclaimerHandle`] from the [`Queue`]'s internal [`Reclaimer`].
    #[inline]
    pub fn reclaimer_handle(&self) -> R::Handle {
        self.reclaimer.local_handle()
    }

    /// Creates a new (thread-local) [`Handle`] for accessing the queue safely.
    #[inline]
    pub fn handle(&self) -> Handle<T, R> {
        Handle { handle: self.reclaimer_handle(), queue: self }
    }

    /// Pushes `elem` to the tail of the [`Queue`] using `handle` to protect any memory records.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that `handle` has been derived from the same [`Queue`] it is used
    /// to access.
    #[inline]
    pub unsafe fn push_unchecked(&self, elem: T, handle: &R::Handle) {
        let mut node = Owned::leak_unprotected(Owned::new(Node::new(elem)));
        let mut guard = handle.clone().guard();
        loop {
            let tail = self.tail.load(&mut guard, Ordering::Acquire).unwrap_unchecked();
            let next = tail.deref().next.load_unprotected(Ordering::Relaxed);

            if next.is_null() {
                match tail.deref().next.compare_exchange(next, node, Self::RELEASE_CAS) {
                    Err(fail) => node = fail.input,
                    Ok(_) => {
                        let _ = self.tail.compare_exchange(tail, node, Self::RELAXED_CAS);
                        return;
                    }
                }
            } else {
                let _ = self.tail.compare_exchange(tail, next, Self::RELEASE_CAS);
            }
        }
    }

    /// Pops an element from the head of the [`Queue`] using `handle` to protect any memory records.
    /// If the [`Queue`] is empty, [`None`] is returned.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that `handle` has been derived from the same [`Queue`] it is used
    /// to access.
    #[inline]
    pub unsafe fn pop_unchecked(&self, handle: &R::Handle) -> Option<T> {
        let mut head_guard = handle.clone().guard();
        let mut next_guard = handle.clone().guard();

        let mut head = self.head.load(&mut head_guard, Ordering::Acquire).unwrap_unchecked();
        while let NotNull(next) = head.deref().next.load(&mut next_guard, Ordering::Relaxed) {
            if let Ok(unlinked) = self.head.compare_exchange(head, next, Self::RELAXED_CAS) {
                let res = ptr::read(&*next.deref().elem);
                handle.clone().retire(unlinked.into_retired());

                return res;
            }

            head = self.head.load(&mut head_guard, Ordering::Acquire).unwrap_unchecked();
        }

        None
    }
}

impl<T, R: GlobalReclaimer> Queue<T, R> {
    /// Pushes `elem` to the end of the [`Queue`].
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.push_unchecked(elem, &R::handle()) }
    }

    /// Pops an element from the head of the [`Queue`] or returns [`None`] if it is empty.
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_unchecked(&R::handle()) }
    }
}

/********** impl Default **************************************************************************/

impl<T, R: GenericReclaimer> Default for Queue<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: GenericReclaimer> Drop for Queue<T, R> {
    #[inline]
    fn drop(&mut self) {
        // this is safe as long as only head the pointer is taken
        let mut curr = self.head.take();
        while let Some(mut node) = curr {
            unsafe { ManuallyDrop::drop(&mut node.elem) };
            curr = node.next.take();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Handle
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Handle<'q, T, R: GenericReclaimer> {
    handle: R::Handle,
    queue: &'q Queue<T, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R: GenericReclaimer> Handle<'_, T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.queue.push_unchecked(elem, &self.handle) };
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.queue.pop_unchecked(&self.handle) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Node<T, R> {
    elem: ManuallyDrop<Option<T>>,
    next: Atomic<Node<T, R>, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R> Node<T, R> {
    #[inline]
    const fn sentinel() -> Self {
        Self { elem: ManuallyDrop::new(None), next: Atomic::null() }
    }

    #[inline]
    const fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(Some(elem)), next: Atomic::null() }
    }
}

fn main() {}
