use crate::traits::*;
use super::stream::*;

use flo_rope::*;

///
/// Trait implemented by types that represent a rope binding
///
/// Ropes are collections of values with optional attributes applied to them.
///
pub trait BoundRope<Cell, Attribute> : Bound<AttributedRope<Cell, Attribute>> 
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    /// Follows the changes to the bound rope as a stream
    fn follow_changes(&self) -> RopeStream<Cell, Attribute>;

    /// Follows the changes to the bound rope as a stream. The stream does not end if the original rope binding is dropped.
    fn follow_changes_retained(&self) -> RopeStream<Cell, Attribute>;
}
