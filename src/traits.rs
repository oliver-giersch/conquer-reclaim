pub unsafe trait Reclaim: Sized + 'static {
    type RecordHeader: Default + Sync + Sized;
}