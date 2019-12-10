use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::conquer_pointer::MaybeNull::{self, NotNull, Null};
use conquer_reclaim::{GenericReclaimer, GlobalReclaimer, Owned, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, conquer_reclaim::typenum::U0>;

type Acq = Ordering::Acquire;
type Rel = Ordering::Release;
type Rlx = Ordering::Relaxed;

const REL_RLX: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Queue<T, R: GenericReclaimer> {
    head: Atomic<Node<T, R>, R>,
    tail: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

impl<T, R: GenericReclaimer> Queue<T, R> {
    #[inline]
    pub fn new() -> Self {
        let sentinel = Owned::leak_unprotected(Owned::new(Node {
            elem: ManuallyDrop::new(None),
            next: Atomic::null(),
        }));

        Self {
            head: unsafe { Atomic::from_unprotected(sentinel) },
            tail: unsafe { Atomic::from_unprotected(sentinel) },
            reclaimer: Default::default(),
        }
    }

    #[inline]
    pub fn handle(&self) -> Handle<T, R> {
        Handle { handle: self.reclaimer.create_local_handle(), queue: self }
    }

    #[inline]
    unsafe fn push_inner(&self, elem: T, handle: R::Handle) {
        let mut node = Owned::leak_unprotected(Owned::new(Node {
            elem: ManuallyDrop::new(Some(elem)),
            next: Atomic::null(),
        }));

        let mut guard = handle.guard();
        loop {
            let tail = self.tail.load(&mut guard, Ordering::Acquire).unwrap_unchecked();
            let next = tail.deref().next.load_unprotected(Ordering::Relaxed);

            if next.is_null() {
                match tail.deref().next.compare_exchange(next, node, REL_RLX) {
                    Err(fail) => node = fail.input,
                    Ok(_) => {
                        let _ = self.tail.compare_exchange(tail, node, REL_RLX);
                        return;
                    }
                }
            } else {
                let _ = self.tail.compare_exchange(tail, next, REL_RLX);
            }
        }
    }

    #[inline]
    unsafe fn pop_inner(&self, handle: R::Handle) -> Option<T> {
        let mut head_guard = handle.clone().guard();
        let mut next_guard = handle.clone().guard();

        let mut head = self.head.load(&mut head_guard, Acq).unwrap_unchecked();
        while let NotNull(next) = head.deref().next.load(&mut next_guard, Ordering::Relaxed) {
            if let Ok(unlinked) = self.head.compare_exchange(head, next, REL_RLX) {
                let res = ptr::read(&*next.deref().elem);
                handle.retire(unlinked.into_retired());

                return res;
            }

            head = self.head.load(&mut head_guard, Acq).unwrap_unchecked();
        }

        None
    }
}

impl<T, R: GlobalReclaimer> Queue<T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.push_inner(elem, R::create_handle()) }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_inner(R::create_handle()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Handle
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Handle<'q, T, R: GenericReclaimer> {
    handle: R::Handle,
    queue: &'q Queue<T, R>,
}

impl<T, R: GenericReclaimer> Handle<'_, T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.queue.push_inner(elem, self.handle.clone()) };
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.queue.pop_inner(self.handle.clone()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

struct Node<T, R> {
    elem: ManuallyDrop<Option<T>>,
    next: Atomic<Node<T, R>, R>,
}

fn main() {}
