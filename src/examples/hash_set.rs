use core::borrow::Borrow;
use core::cmp::{
    Ord,
    Ordering::{Equal, Greater},
};
use core::hash::{BuildHasher, Hash, Hasher};
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::sync::atomic::Ordering;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::Arc;
    } else {
        use alloc::sync::Arc;
    }
}

use crate::{ProtectExt, ReclaimRef, ReclaimThreadState};

type Atomic<T, R> = crate::Atomic<T, R, 1>;
type Owned<T, R> = crate::Owned<T, R, 1>;

type FusedProtected<T, G> = crate::fused::FusedProtected<T, G, 1>;
type FusedShared<T, G> = crate::fused::FusedShared<T, G, 1>;
type FusedSharedRef<'g, T, G> = crate::fused::FusedSharedRef<'g, T, G, 1>;

type AssocGuard<T, R> = <<R as ReclaimRef<T>>::ThreadState as ReclaimThreadState<T>>::Guard;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ArcHashSet
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ArcHashSet<T, R: ReclaimRef<Node<T, R>>, S> {
    inner: Arc<HashSet<T, R, S>>,
    thread_state: ManuallyDrop<R::ThreadState>,
}

impl<T, R: ReclaimRef<Node<T, R>>, S> ArcHashSet<T, R, S>
where
    T: Hash + Ord,
    R: ReclaimRef<Node<T, R>>,
    S: BuildHasher,
{
    #[inline]
    pub fn with(hash_builder: S, buckets: usize, reclaimer: R) -> Self {
        let inner = Arc::new(HashSet::with(hash_builder, buckets, reclaimer));
        let thread_state = unsafe { inner.reclaimer.build_thread_state_unchecked() };
        Self { inner, thread_state: ManuallyDrop::new(thread_state) }
    }

    #[inline]
    pub fn insert(&self, elem: T) -> bool {
        unsafe { self.inner.insert(elem, &self.thread_state) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// HashSetRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HastSetRef<'a, T, R: ReclaimRef<Node<T, R>>, S> {
    hash_set: &'a HashSet<T, R, S>,
    thread_state: R::ThreadState,
}

/********** impl inherent *************************************************************************/

impl<'a, T, R: ReclaimRef<Node<T, R>>, S> HastSetRef<'a, T, R, S>
where
    T: Hash + Ord,
    R: ReclaimRef<Node<T, R>>,
    S: BuildHasher,
{
    #[inline]
    pub fn new(hash_set: &'a HashSet<T, R, S>) -> Self {
        Self {
            hash_set,
            thread_state: unsafe { hash_set.reclaimer.build_thread_state_unchecked() },
        }
    }

    #[inline]
    pub fn insert(&self, elem: T) -> bool {
        unsafe { self.hash_set.insert(elem, &self.thread_state) }
    }

    #[inline]
    pub fn remove<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        unsafe { self.hash_set.remove(value, &self.thread_state) }
    }

    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        unsafe { self.hash_set.contains(value, &self.thread_state) }
    }

    #[inline]
    pub fn get<Q>(&self, value: &Q) -> Option<SharedRef<T, R>>
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        unsafe { self.hash_set.get(value, &self.thread_state) }
    }
}

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
    pub unsafe fn insert(&self, elem: T, thread_state: &R::ThreadState) -> bool {
        let mut prev = thread_state.build_guard();
        let curr = thread_state.build_guard();
        let next = thread_state.build_guard();

        let node = thread_state.alloc_owned(Node { elem, next: Atomic::null() });
        let elem = &node.elem;
        let set = &self.buckets[self.make_hash(elem)];

        return set.insert_node(node, thread_state, &mut prev, curr, next);
    }

    #[inline]
    pub unsafe fn remove<Q>(&self, value: &Q, thread_state: &R::ThreadState) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut prev = thread_state.build_guard();
        let curr = thread_state.build_guard();
        let next = thread_state.build_guard();

        let set = &self.buckets[self.make_hash(value)];
        set.remove_node(value, thread_state, &mut prev, curr, next)
    }

    #[inline]
    pub unsafe fn contains<Q>(&self, value: &Q, thread_state: &R::ThreadState) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut prev = thread_state.build_guard();
        let curr = thread_state.build_guard();
        let next = thread_state.build_guard();

        let set = &self.buckets[self.make_hash(value)];
        match set.find(value, thread_state, &mut prev, curr, next) {
            FindResult::Found { .. } => true,
            _ => false,
        }
    }

    #[inline]
    pub unsafe fn get<Q>(&self, value: &Q, thread_state: &R::ThreadState) -> Option<SharedRef<T, R>>
    where
        T: Borrow<Q>,
        Q: Hash + Ord,
    {
        let mut prev = thread_state.build_guard();
        let curr = thread_state.build_guard();
        let next = thread_state.build_guard();

        let set = &self.buckets[self.make_hash(value)];
        match set.find(value, thread_state, &mut prev, curr, next) {
            FindResult::Found { curr, .. } => Some(SharedRef { shared: curr }),
            FindResult::Insert { .. } => None,
        }
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
// SharedRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct SharedRef<T, R: ReclaimRef<Node<T, R>>> {
    shared: FusedShared<Node<T, R>, AssocGuard<Node<T, R>, R>>,
}

/********** impl Deref ****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Deref for SharedRef<T, R> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &self.shared.as_shared().as_ref().elem }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Node<T, R: ReclaimRef<Self>> {
    elem: T,
    next: Atomic<Self, R::Reclaim>,
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

    const ACQ: Ordering = Ordering::Acquire;
    const RLX: Ordering = Ordering::Relaxed;
    const ACQ_RLX: (Ordering, Ordering) = (Ordering::Acquire, Ordering::Relaxed);
    const REL_RLX: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);

    #[inline]
    fn new() -> Self {
        Self { head: Atomic::null() }
    }

    #[inline]
    unsafe fn insert_node(
        &self,
        mut node: Owned<Node<T, R>, R::Reclaim>,
        thread_state: &R::ThreadState,
        prev_guard: &mut AssocGuard<Node<T, R>, R>,
        mut curr_guard: AssocGuard<Node<T, R>, R>,
        mut next_guard: AssocGuard<Node<T, R>, R>,
    ) -> bool {
        loop {
            let elem = &node.elem;
            match self.find(elem, thread_state, prev_guard, curr_guard, next_guard) {
                FindResult::Insert { prev, next, guard } => {
                    node.next.store(next.as_protected(), Ordering::Relaxed);
                    match prev.as_ref().compare_exchange(next.as_protected(), node, Self::REL_RLX) {
                        Ok(_) => return true,
                        Err(fail) => {
                            node = fail.input;
                            curr_guard = next.into_guard();
                            next_guard = guard;
                        }
                    };
                }
                FindResult::Found { .. } => {
                    return false;
                }
            }
        }
    }

    #[inline]
    unsafe fn remove_node<Q>(
        &self,
        value: &Q,
        thread_state: &R::ThreadState,
        prev_guard: &mut AssocGuard<Node<T, R>, R>,
        mut curr_guard: AssocGuard<Node<T, R>, R>,
        mut next_guard: AssocGuard<Node<T, R>, R>,
    ) -> bool
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        loop {
            match self.find(value, thread_state, prev_guard, curr_guard, next_guard) {
                FindResult::Insert { .. } => return false,
                FindResult::Found { prev, curr, next } => {
                    let next_ref = &curr.as_shared().as_ref().next;
                    let next_before = next.as_protected();
                    let next_marked = next_before.set_tag(Self::DELETE_TAG);

                    if next_ref.compare_exchange(next_before, next_marked, Self::ACQ_RLX).is_err() {
                        curr_guard = curr.into_guard();
                        next_guard = next.into_guard();
                        continue;
                    }

                    let curr_ref = curr.as_shared();
                    match prev.as_ref().compare_exchange(curr_ref, next_before, Self::REL_RLX) {
                        Ok(unlinked) => thread_state.retire_record(unlinked.into_retired()),
                        Err(_) => {
                            curr_guard = curr.into_guard();
                            next_guard = next.into_guard();
                            let _ =
                                self.find(value, thread_state, prev_guard, curr_guard, next_guard);
                        }
                    }

                    return true;
                }
            }
        }
    }

    unsafe fn find<'set, 'g, Q>(
        &'set self,
        value: &Q,
        thread_state: &R::ThreadState,
        prev_guard: &'g mut AssocGuard<Node<T, R>, R>,
        mut curr_guard: AssocGuard<Node<T, R>, R>,
        mut next_guard: AssocGuard<Node<T, R>, R>,
    ) -> FindResult<'set, 'g, T, R>
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        'retry: loop {
            let mut prev = Previous::Set(&self.head);
            loop {
                match curr_guard.protect_fused(prev.as_ref(), Self::ACQ).into_fused_shared() {
                    Ok(curr_fused) => {
                        let (curr, tag) = curr_fused.as_shared().split_tag();
                        if tag == Self::DELETE_TAG {
                            curr_guard = curr_fused.into_guard();
                            continue 'retry;
                        }

                        let next_ref = &curr.as_ref().next;

                        let expected = next_ref.load_raw(Self::RLX);
                        match next_guard.protect_fused_if_equal(next_ref, expected, Self::ACQ) {
                            Err((next, _)) => {
                                curr_guard = curr_fused.into_guard();
                                next_guard = next;
                                continue 'retry;
                            }
                            Ok(next_fused) => {
                                if prev.as_ref().load_raw(Self::RLX) != curr.into_marked_ptr() {
                                    curr_guard = curr_fused.into_guard();
                                    next_guard = next_fused.into_guard();
                                    continue 'retry;
                                }

                                let (next, tag) = next_fused.as_protected().split_tag();
                                if tag == Self::DELETE_TAG {
                                    match prev.as_ref().compare_exchange(curr, next, Self::REL_RLX)
                                    {
                                        Ok(unlinked) => {
                                            thread_state.retire_record(unlinked.into_retired())
                                        }
                                        Err(_) => {
                                            curr_guard = curr_fused.into_guard();
                                            next_guard = next_fused.into_guard();
                                            continue 'retry;
                                        }
                                    }
                                } else {
                                    match curr.as_ref().elem.borrow().cmp(value) {
                                        Equal => {
                                            return FindResult::Found {
                                                prev,
                                                curr: curr_fused,
                                                next: next_fused,
                                            }
                                        }
                                        Greater => {
                                            return FindResult::Insert {
                                                prev,
                                                next: curr_fused.into_fused_protected(),
                                                guard: next_fused.into_guard(),
                                            }
                                        }
                                        _ => {}
                                    };
                                }

                                // SAFETY: ..
                                let prev_guard = &mut *(prev_guard as *mut _);
                                let (prev_fused, free) = curr_fused.transfer_to_ref(prev_guard);
                                curr_guard = free;
                                next_guard = next_fused.into_guard();
                                prev = Previous::Guarded(prev_fused);
                            }
                        }
                    }
                    Err((next, _)) => return FindResult::Insert { prev, next, guard: next_guard },
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// FindResult
////////////////////////////////////////////////////////////////////////////////////////////////////

enum FindResult<'set, 'g, T, R: ReclaimRef<Node<T, R>>> {
    Found {
        prev: Previous<'set, 'g, T, R>,
        curr: FusedShared<Node<T, R>, AssocGuard<Node<T, R>, R>>,
        next: FusedProtected<Node<T, R>, AssocGuard<Node<T, R>, R>>,
    },
    Insert {
        prev: Previous<'set, 'g, T, R>,
        next: FusedProtected<Node<T, R>, AssocGuard<Node<T, R>, R>>,
        guard: AssocGuard<Node<T, R>, R>,
    },
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Previous
////////////////////////////////////////////////////////////////////////////////////////////////////

enum Previous<'set, 'g, T, R: ReclaimRef<Node<T, R>>> {
    Set(&'set Atomic<Node<T, R>, R::Reclaim>),
    Guarded(FusedSharedRef<'g, Node<T, R>, AssocGuard<Node<T, R>, R>>),
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Previous<'_, '_, T, R> {
    #[inline]
    unsafe fn as_ref(&self) -> &Atomic<Node<T, R>, R::Reclaim> {
        match self {
            Previous::Set(head) => *head,
            Previous::Guarded(shared) => &shared.as_shared().as_ref().next,
        }
    }
}
