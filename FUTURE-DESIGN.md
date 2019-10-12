# Future API Design

## RetiredStorage

```rust
pub trait StoreRetired {
    type Reclaimer: Reclaim;

    unsafe fn retire(&self, record: Retired<Self::Reclaimer>);
}
```

## Protect

```rust
pub unsafe trait Protect: Sized + Clone {
    type Reclaimer: Reclaim;

    fn release(&mut self);

    fn protect<T, const N: usize>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering
    ) -> MarkedOption<Shared<T, Self::Reclaimer, N>>;
    
    fn protect_if_equal<T, const N: usize>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering
    ) -> Result<MarkedOption<Shared<T, Self::Reclaimer>>, NotEqual>;
}
```

## Reclaim

```rust
pub unsafe trait Reclaim: Sized {
    type Allocator: AllocRef;
    type Guard<'local, 'global: 'local>:
        Protect<Reclaimer = Self> + 'local + 'global;
    type RecordHeader: Default + Sync + Sized;
}
```
