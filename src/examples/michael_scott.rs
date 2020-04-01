use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

use crate::{LocalState, Maybe, Reclaim};

type Atomic<T, R> = crate::Atomic<T, R, crate::typenum::U0>;
type Owned<T, R> = crate::Owned<T, R, crate::typenum::U0>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Queue<T, R: Reclaim> {
    head: Atomic<Node<T, R>, R>,
    tail: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Queue<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Release, Relaxed);

    #[inline]
    pub fn new() -> Self {
        let sentinel = Owned::leak_storable(Owned::new(Node::sentinel()));
        Self {
            head: Atomic::from(sentinel),
            tail: Atomic::from(sentinel),
            reclaimer: Default::default(),
        }
    }

    #[inline]
    pub unsafe fn push_unchecked(&self, elem: T, local_state: &R::LocalState) {
        let node = Owned::leak_storable(Owned::new(Node::new(elem)));
        let mut guard = local_state.build_guard();
        loop {
            let tail = self.tail.load(&mut guard, Acquire);
            let next = tail.deref().next.load_unprotected(Relaxed);

            if next.is_null() {
                if tail.deref().next.compare_exchange(next, node, Self::RELEASE_CAS).is_ok() {
                    let _ = self.tail.compare_exchange(tail, node, Self::RELEASE_CAS);
                    return;
                }
            } else {
                // safety: `next` can be safely used as store argument here, it will only actually
                // be inserted if the CAS succeeds, in which case it could not already have been
                // popped and retired/reclaimed
                let _ = self.tail.compare_exchange(tail, next.assume_storable(), Self::RELEASE_CAS);
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
            match self.head.compare_exchange(head, next, Self::RELEASE_CAS) {
                Ok(unlinked) => {
                    // safety: `elem` is logically and uniquely taken out (consumed) here
                    let res = Some(ptr::read(next.as_ref().elem.as_ptr()));
                    // safety: The previous head is no longer visible for other threads and since
                    // `elem` won't be dropped when the node is reclaimed it doesn't matter if it
                    // outlives any internal references.
                    local_state.retire_record(unlinked.into_retired_unchecked());

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

impl<T, R: Reclaim> Default for Queue<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim> Drop for Queue<T, R> {
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

pub struct QueueRef<'q, T, R: Reclaim> {
    queue: &'q Queue<T, R>,
    reclaimer_local_state: R::LocalState,
}

/********** impl inherent *************************************************************************/

impl<'q, T, R: Reclaim> QueueRef<'q, T, R> {
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

struct Node<T, R> {
    elem: MaybeUninit<T>,
    next: Atomic<Self, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Node<T, R> {
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
