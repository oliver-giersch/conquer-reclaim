use crate::record::Record;
use crate::traits::{Reclaim, ReclaimBase};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocReclaimBase (alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub type AssocReclaimBase<T, R> = <R as Reclaim<T>>::Base;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocHeader (alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub type AssocHeader<T, R> = <AssocReclaimBase<T, R> as ReclaimBase>::Header;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocRecord (alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) type AssocRecord<T, R> = Record<<<R as Reclaim<T>>::Base as ReclaimBase>::Header, T>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// RetiredRecord (alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) type RetiredRecord<R> = Record<<R as ReclaimBase>::Header, <R as ReclaimBase>::Retired>;