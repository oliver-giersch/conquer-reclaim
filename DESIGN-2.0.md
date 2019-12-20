# API Design

## Protect

```rust
pub unsafe trait Protect {
    type Reclaimer;

    fn release(&mut self);

    fn protect<T, const N: usize>(
        &mut self,
        src: Atomic<T, Self::Reclaimer, N>,
        order: Ordering
    ) -> MaybeNull<Shared<T, Self::Reclaimer, N>>;

    fn protect_if_equal<T, const N: usize>(
        &mut self,
        src: Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering
    ) -> Result<MaybeNull<Shared<T, Self::Reclaimer, N>>, NotEqualError>;
}
```

## Reclaim

```rust
pub unsafe trait Reclaim: Default + Sized {
    type Header: Default + Sync + Send + Sized;
    type Ref<'global>: LocalRef<Reclaimer = Self> + 'global;

    fn new() -> Self;
}
```

## LocalRef

```rust
pub trait LocalRef: Clone {
    type Guard: Protect<Reclaimer = Self::Reclaimer>;
    type Reclaimer: Reclaim;
    
    fn from_owned(global: &Self::Reclaimer) -> Self where Self: 'static;
    fn from_ref(global: &Self::Reclaimer) -> Self;
    unsafe fn from_raw(global: *const Self::Reclaimer) -> Self;

    fn into_guard(self) -> Self::Guard;
    unsafe fn retire(self, retired: Retired<Self::Reclaimer>);
}
```
