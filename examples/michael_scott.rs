use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::conquer_pointer::MaybeNull::{self, NotNull, Null};
use conquer_reclaim::{GenericReclaimer, GlobalReclaimer, Owned, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, conquer_reclaim::typenum::U0>;

type Acq = Ordering::Acquire;
type Rel = Ordering::Release;
type Rlx = Ordering::Relaxed;

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
            let tail = self.tail.load(&mut guard, Ordering::Acquire).unwrap();
            let next = tail.deref().next.load_unprotected(Ordering::Relaxed);

            if next.is_null() {
                match tail.deref().next.compare_exchange(next, node, Rel, Rlx) {
                    Err(fail) => node = fail.input,
                    Ok(_) => {
                        let _ = self.tail.compare_exchange(tail, node, Rel, Rlx);
                        return;
                    }
                }
            } else {
                let _ =
                    self.tail.compare_exchange(tail, next, Ordering::Release, Ordering::Relaxed);
            }
        }
    }

    #[inline]
    unsafe fn pop_inner(&self, handle: R::Handle) -> Option<T> {
        let mut head_guard = handle.clone().guard();
        let mut next_guard = handle.clone().guard();

        let mut head = self.head.load(&mut head_guard, Acq).unwrap();
        while let NotNull(next) = head.deref().next.load(&mut next_guard, Rlx) {
            if let Ok(unlinked) = self.head.compare_exchange(head, next, Rel, Rlx) {
                let res = ptr::read(&*next.deref().elem);
                handle.retire(unlinked.into_retired());
                return res;
            }

            head = self.head.load(&mut head_guard, Acq).unwrap();
        }

        None
    }
}

impl<T, R: GlobalReclaimer> Queue<T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.push_inner(elem, R::Handle::default()) }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_inner(R::Handle::default()) }
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
