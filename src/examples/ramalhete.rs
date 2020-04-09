use core::cell::UnsafeCell;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::sync::atomic::{AtomicU32, AtomicU8};

use crate::Reclaim;

type Atomic<T, R> = crate::Atomic<T, R, crate::typenum::U0>;

const NODE_SIZE: usize = 1024;

pub struct Queue<T, R: Reclaim> {
    head: Atomic<Node<T, R>, R>,
    tail: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

impl<T, R: Reclaim> Queue<T, R> {
    pub unsafe fn push_unchecked(&self, elem: T, local_state: &R::LocalState) {
        todo!()
    }
}

pub struct QueueRef<'q, T, R: Reclaim> {
    queue: &'q Queue<T, R>,
    reclaim_local_state: ManuallyDrop<R::LocalState>,
}

impl<'q, T, R: Reclaim> QueueRef<'q, T, R> {
    #[inline]
    pub fn new(queue: &'q Queue<T, R>) -> Self {
        Self {
            queue,
            reclaim_local_state: ManuallyDrop::new(unsafe { queue.reclaimer.build_local_state() }),
        }
    }
}

#[repr(C)]
struct Node<T, R: Reclaim> {
    pop_idx: AtomicU32,
    push_idx: AtomicU32,
    slots: [Slot<T>; NODE_SIZE],
    next: Atomic<Self, R>,
}

struct Slot<T> {
    cell: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
}
