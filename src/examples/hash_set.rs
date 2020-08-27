use core::borrow::Borrow;
use core::cmp::{
    Ord,
    Ordering::{Equal, Greater},
};
use core::hash::{BuildHasher, Hash, Hasher};
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::sync::atomic::Ordering;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::Arc;
    } else {
        use alloc::boxed::Box;
        use alloc::sync::Arc;
    }
}

use crate::{Maybe, Protect, ProtectExt, ReclaimRef, ReclaimThreadState};

type Atomic<T, R> = crate::Atomic<T, R, 1>;
type Owned<T, R> = crate::Owned<T, R, 1>;
type Protected<'g, T, R> = crate::Protected<'g, T, R, 1>;
type Shared<'g, T, R> = crate::Shared<'g, T, R, 1>;

type AssocGuard<T, R> = <<R as ReclaimRef<T>>::ThreadState as ReclaimThreadState<T>>::Guard;

////////////////////////////////////////////////////////////////////////////////////////////////////
// HashSet
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HashSet<T, R: ReclaimRef<Node<T, R>>, S> {
    buckets: Box<[OrderedSet<T, R>]>,
    reclaimer: R,
    hash_builder: S,
}

/*********** impl inherent ************************************************************************/

impl<T, R, S> HashSet<T, R, S>
where
    T: Hash + Ord,
    R: ReclaimRef<Node<T, R>>,
    S: BuildHasher,
{
    #[inline]
    pub fn with(hash_builder: S, buckets: usize, reclaimer: R) -> Self {
        assert!(buckets > 0, "hash set needs to contain at least one bucket");
        Self { buckets: (0..buckets).map(|_| OrderedSet::new()).collect(), reclaimer, hash_builder }
    }

    #[inline]
    pub unsafe fn contains<Q>(&self, value: &Q, thread_state: &R::ThreadState) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut guards = &mut Guards {
            prev: thread_state.build_guard(),
            curr: thread_state.build_guard(),
            next: thread_state.build_guard(),
        };

        let set = &self.buckets[self.make_hash(value)];
        set.remove_node(value, guards, thread_state)
    }

    #[inline]
    fn make_hash<Q>(&self, value: &Q) -> usize
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut state = self.hash_builder.build_hasher();
        value.hash(&mut state);
        (state.finish() % self.buckets.len() as u64) as usize
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// HashSetRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HastSetRef<'a, T, R: ReclaimRef<Node<T, R>>, S> {
    hash_set: &'a HashSet<T, R, S>,
    thread_state: ManuallyDrop<R::ThreadState>,
}

/********** impl inherent *************************************************************************/

impl<'a, T, R: ReclaimRef<Node<T, R>>, S> HastSetRef<'a, T, R, S> {
    #[inline]
    pub fn new(hash_set: &'a HashSet<T, R, S>) -> Self {
        Self {
            hash_set,
            thread_state: ManuallyDrop::new(unsafe {
                hash_set.reclaimer.build_thread_state_unchecked()
            }),
        }
    }

    pub fn get<Q>(&mut self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut guards = &mut Guards::new(&self.thread_state);
        todo!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Node<T, R: ReclaimRef<Self>> {
    elem: T,
    next: Atomic<Self, R::Reclaim>,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef<Self>> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem, next: Atomic::null() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guards
////////////////////////////////////////////////////////////////////////////////////////////////////

struct Guards<T, R: ReclaimRef<Node<T, R>>> {
    prev: AssocGuard<Node<T, R>, R>,
    curr: AssocGuard<Node<T, R>, R>,
    next: AssocGuard<Node<T, R>, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Guards<T, R> {
    #[inline]
    fn new(thread_state: &R::ThreadState) -> Self {
        Self {
            prev: thread_state.build_guard(),
            curr: thread_state.build_guard(),
            next: thread_state.build_guard(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// OrderedSet
////////////////////////////////////////////////////////////////////////////////////////////////////

struct OrderedSet<T, R: ReclaimRef<Node<T, R>>> {
    head: Atomic<Node<T, R>, R::Reclaim>,
}

/********** impl inherent *************************************************************************/

impl<T, R> OrderedSet<T, R>
where
    T: Ord,
    R: ReclaimRef<Node<T, R>>,
{
    const DELETE_TAG: usize = 0x1;
    const ACQ_RLX: (Ordering, Ordering) = (Ordering::Acquire, Ordering::Relaxed);
    const REL_RLX: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);

    #[inline]
    const fn new() -> Self {
        Self { head: Atomic::null() }
    }

    unsafe fn insert_node(
        &self,
        value: T,
        guards: &mut Guards<T, R>,
        thread_state: &R::ThreadState,
    ) -> bool {
        let mut node = thread_state.alloc_owned(Node::new(value));
        loop {
            let elem = &node.elem;
            match self.find(elem, guards, thread_state) {
                FindResult::Insert { prev, next } => {
                    node.next.store(next, Ordering::Relaxed);
                    match (*prev).compare_exchange(next, node, Self::REL_RLX) {
                        Ok(_) => return true,
                        Err(e) => {
                            node = e.input;
                        }
                    };
                }
                _ => return false,
            }
        }
    }

    unsafe fn remove_node<Q>(
        &self,
        value: &Q,
        guards: &mut Guards<T, R>,
        thread_state: &R::ThreadState,
    ) -> bool
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        loop {
            match self.find(value, guards, thread_state) {
                FindResult::Insert { .. } => return false,
                FindResult::Found { prev, curr, next } => {
                    let next_tag = next.set_tag(Self::DELETE_TAG);
                    if curr.as_ref().next.compare_exchange(next, next_tag, Self::ACQ_RLX).is_err() {
                        continue;
                    }

                    match (*prev).compare_exchange(curr, next, Self::REL_RLX) {
                        Ok(unlinked) => thread_state.retire_record(unlinked.into_retired()),
                        Err(_) => {
                            let _ = self.find(value, guards, thread_state);
                        }
                    }

                    return true;
                }
            }
        }
    }

    unsafe fn find<'set, 'g, Q>(
        &'set self,
        val: &Q,
        guards: &'g mut Guards<T, R>,
        thread_state: &R::ThreadState,
    ) -> FindResult<'g, T, R>
    where
        R: 'g,
        T: Borrow<Q> + 'g,
        Q: Ord,
    {
        'retry: loop {
            let mut prev = &self.head;
            while let Maybe::Some(fused) = guards.curr.protect_fused_ref(prev, Ordering::Acquire) {
                let (curr, tag) = fused.as_shared().split_tag();
                if tag == Self::DELETE_TAG {
                    continue 'retry;
                }

                // SAFETY: de-referencing curr is safe due to the `Acquire` ordering of its load
                let next_ref = &curr.as_ref().next;

                let expected = next_ref.load_raw(Ordering::Relaxed);
                match next_ref.load_if_equal(expected, &mut guards.next, Ordering::Acquire) {
                    Err(_) => continue 'retry,
                    Ok(next) => {
                        if prev.load_raw(Ordering::Relaxed) != curr.into_marked_ptr() {
                            continue 'retry;
                        }

                        let (next, tag) = next.split_tag();
                        if tag == Self::DELETE_TAG {
                            match prev.compare_exchange(curr, next, Self::REL_RLX) {
                                // SAFETY: ...
                                Ok(unlinked) => thread_state.retire_record(unlinked.into_retired()),
                                Err(_) => continue 'retry,
                            }
                        } else {
                            // SAFETY: using `cast` on the returned values is an unfortunate escape
                            // hatch, which is required because the compiler is not smart enough to
                            // recognize that returning these values is actually sound
                            match curr.as_ref().elem.borrow().cmp(val) {
                                Equal => {
                                    return FindResult::Found {
                                        prev,
                                        curr: fused.into_shared().cast(),
                                        next: next.cast(),
                                    }
                                }
                                Greater => {
                                    return FindResult::Insert {
                                        prev,
                                        next: fused.into_shared().into_protected().cast(),
                                    }
                                }
                                _ => {}
                            }

                            // transfering the responsibility for protecting the current node to
                            // `prev` allows using `curr` to be used again in the next iteration
                            let curr = guards.prev.adopt(fused);
                            prev = &curr.as_ref().next;
                        }
                    }
                }
            }

            return FindResult::Insert { prev, next: Protected::null() };
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// FindResult
////////////////////////////////////////////////////////////////////////////////////////////////////

enum FindResult<'g, T, R: ReclaimRef<Node<T, R>>> {
    Found {
        prev: *const Atomic<Node<T, R>, R::Reclaim>,
        curr: Shared<'g, Node<T, R>, R::Reclaim>,
        next: Protected<'g, Node<T, R>, R::Reclaim>,
    },
    Insert {
        prev: *const Atomic<Node<T, R>, R::Reclaim>,
        next: Protected<'g, Node<T, R>, R::Reclaim>,
    },
}
