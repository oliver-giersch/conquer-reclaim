use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr;
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::Arc;
    } else {
        use alloc::sync::Arc;
    }
}

use crate::{AssocReclaim, Maybe, ReclaimLocalState, ReclaimRef, Retire};

type Atomic<T, R> = crate::Atomic<T, R, crate::typenum::U0>;
type Owned<T, R> = crate::Owned<T, R, crate::typenum::U0>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ArcQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ArcQueue<T, R: ReclaimRef> {
    inner: Arc<Queue<T, R>>,
    reclaim_local_state: ManuallyDrop<R::LocalState>,
}

/*********** impl Send ****************************************************************************/

unsafe impl<T, R: ReclaimRef> Send for ArcQueue<T, R> {}

/*********** impl Clone ***************************************************************************/

impl<T, R: ReclaimRef> Clone for ArcQueue<T, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            reclaim_local_state: ManuallyDrop::new(unsafe {
                self.inner.reclaimer.build_local_state()
            }),
        }
    }
}

/*********** impl inherent ************************************************************************/

impl<T, R: ReclaimRef> ArcQueue<T, R> {
    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        let inner = Arc::new(Queue::<_, R>::with_reclaimer(reclaimer));
        let reclaim_local_state = unsafe { inner.reclaimer.build_local_state() };
        Self { inner, reclaim_local_state: ManuallyDrop::new(reclaim_local_state) }
    }
}

impl<T, R: ReclaimRef + Default> ArcQueue<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef<Item = Node<T, R>>> ArcQueue<T, R>
where
    AssocReclaim<R>: Retire<Node<T, R>>,
{
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.inner.push_unchecked(elem, &*self.reclaim_local_state) }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.inner.pop_unchecked(&self.reclaim_local_state) }
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef + Default> Default for ArcQueue<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef> Drop for ArcQueue<T, R> {
    #[inline]
    fn drop(&mut self) {
        // safety: Drop Local state before the `Arc`, because it may hold a pointer into it.
        unsafe { ManuallyDrop::drop(&mut self.reclaim_local_state) };
    }
}

/********** impl From (Queue) *********************************************************************/

impl<T, R: ReclaimRef> From<Queue<T, R>> for ArcQueue<T, R> {
    #[inline]
    fn from(queue: Queue<T, R>) -> Self {
        let inner = Arc::new(queue);
        let reclaim_local_state = ManuallyDrop::new(unsafe { inner.reclaimer.build_local_state() });

        Self { inner, reclaim_local_state }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A concurrent unbounded lock-free multi-producer/multi-consumer FIFO queue.
///
/// The implementation is based on an algorithm by Michael Scott and Maged
/// Michael.
pub struct Queue<T, R: ReclaimRef> {
    head: Atomic<Node<T, R>, R::Reclaim>,
    tail: Atomic<Node<T, R>, R::Reclaim>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef> Queue<T, R> {
    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        let sentinel = Owned::<_, R::Reclaim>::leak(Owned::new(Node::sentinel()));
        Self {
            head: Atomic::<_, R::Reclaim>::from(sentinel),
            tail: Atomic::<_, R::Reclaim>::from(sentinel),
            reclaimer,
        }
    }
}

impl<T, R: ReclaimRef + Default> Queue<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef<Item = Node<T, R>>> Queue<T, R>
where
    AssocReclaim<R>: Retire<Node<T, R>>,
{
    const REL_RLX: (Ordering, Ordering) = (Release, Relaxed);

    #[inline]
    pub unsafe fn push_unchecked(&self, elem: T, local_state: &R::LocalState) {
        let node = Owned::leak(Owned::new(Node::new(elem)));
        let mut guard = local_state.build_guard();
        loop {
            let tail = self.tail.load(&mut guard, Acquire);
            let next = tail.deref().next.load_unprotected(Relaxed);

            if next.is_null() {
                if tail.deref().next.compare_exchange(next, node, Self::REL_RLX).is_ok() {
                    let _ = self.tail.compare_exchange(tail, node, Self::REL_RLX);
                    return;
                }
            } else {
                // safety: `next` can be safely used as store argument here, it will only actually
                // be inserted if the CAS succeeds, in which case it could not already have been
                // popped and retired/reclaimed
                let _ = self.tail.compare_exchange(tail, next.assume_storable(), Self::REL_RLX);
            }
        }
    }

    #[inline]
    pub unsafe fn pop_unchecked(&self, local_state: &R::LocalState) -> Option<T> {
        let mut head_guard = local_state.build_guard();
        let mut next_guard = local_state.build_guard();

        // safety: head can never be null
        let mut head = self.head.load(&mut head_guard, Acquire).shared_unchecked();
        while let Maybe::Some(next) = head.as_ref().next.load(&mut next_guard, Acquire).shared() {
            match self.head.compare_exchange(head, next, Self::REL_RLX) {
                Ok(unlinked) => {
                    // safety: `elem` is logically and uniquely taken out (consumed) here
                    let res = Some(ptr::read(next.as_ref().elem.as_ptr()));
                    // safety: The previous head is no longer visible for other threads and since
                    // `elem` won't be dropped when the node is reclaimed it doesn't matter if it
                    // outlives any internal references.
                    local_state.retire_record(unlinked);

                    return res;
                }
                // safety: head can never be null
                Err(_) => head = self.head.load(&mut head_guard, Acquire).shared_unchecked(),
            }
        }

        None
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef + Default> Default for Queue<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef> Drop for Queue<T, R> {
    fn drop(&mut self) {
        unsafe {
            // safety: As long as tail is left in place, no node can be freed twice,
            // head node is always the sentinel
            let mut head = self.head.take().unwrap();
            // safety: As long as tail is left in place, no node can be freed twice
            let mut curr = head.next.take();
            while let Some(mut node) = curr {
                // safety: All nodes after the sentinel contained initialized memory
                node.elem.as_mut_ptr().drop_in_place();
                curr = node.next.take();
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// QueueRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct QueueRef<'q, T, R: ReclaimRef> {
    queue: &'q Queue<T, R>,
    reclaimer_local_state: R::LocalState,
}

/********** impl inherent *************************************************************************/

impl<'q, T, R: ReclaimRef<Item = Node<T, R>>> QueueRef<'q, T, R>
where
    AssocReclaim<R>: Retire<Node<T, R>>,
{
    #[inline]
    pub fn new(queue: &'q Queue<T, R>) -> Self {
        Self { queue, reclaimer_local_state: unsafe { queue.reclaimer.build_local_state() } }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.queue.push_unchecked(elem, &self.reclaimer_local_state) }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.queue.pop_unchecked(&self.reclaimer_local_state) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Node<T, R: ReclaimRef> {
    elem: MaybeUninit<T>,
    next: Atomic<Self, R::Reclaim>,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef> Node<T, R> {
    /// Creates a sentinel [`Node`] with an uninitialized elem.
    #[inline]
    fn sentinel() -> Self {
        Self { elem: MaybeUninit::uninit(), next: Default::default() }
    }

    /// Creates a new [`Node`] containing `elem`.
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: MaybeUninit::new(elem), next: Default::default() }
    }
}
