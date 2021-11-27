use crate::rope_binding::bound_rope::*;
use crate::rope_binding::rope_binding::*;

use futures::prelude::*;
use futures::stream;
use futures::task::{Poll};

use flo_rope::*;

use std::iter;
use std::collections::{VecDeque};

///
/// Extension methods that can be applied to any bound rope
///
pub trait BoundRopeExt<Cell, Attribute> 
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    ///
    /// Returns a new rope that concatenates the contents of this rope and another one
    ///
    fn concat<OtherRope: BoundRope<Cell, Attribute>>(&self, other: &OtherRope) -> RopeBinding<Cell, Attribute>;
}

impl<Cell, Attribute, TRope> BoundRopeExt<Cell, Attribute> for TRope
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default,
TRope:      BoundRope<Cell, Attribute> {
    fn concat<OtherRope: BoundRope<Cell, Attribute>>(&self, other: &OtherRope) -> RopeBinding<Cell, Attribute> {
        // Follow the left and right-hand streams
        let mut follow_left     = Some(self.follow_changes());
        let mut follow_right    = Some(other.follow_changes());

        // Concatenator and pending values
        let mut pending         = VecDeque::new();
        let mut concatenator    = RopeConcatenator::new();

        // Create a new polling stream that concatenates the two sides
        let concat_stream       = stream::poll_fn(move |ctxt| {
            if let Some(next) = pending.pop_front() {
                // Always process pending changes first
                return Poll::Ready(Some(next));
            }

            // Process left-hand side changes, if there are any
            let poll_left = follow_left.as_mut().map(|left| left.poll_next_unpin(ctxt));

            match poll_left {
                None                            => { }
                Some(Poll::Pending)             => { }
                Some(Poll::Ready(None))         => { follow_left = None; }
                Some(Poll::Ready(Some(action))) => {
                    // Send to the LHS 
                    for action in concatenator.send_left(iter::once(action)) {
                        pending.push_back(action);
                    }

                    // Return any pending actions
                    if let Some(next) = pending.pop_front() { return Poll::Ready(Some(next)); }
                }
            }

            // Process right-hand side changes, if there are any
            let poll_right = follow_right.as_mut().map(|right| right.poll_next_unpin(ctxt));

            match poll_right {
                None                            => { }
                Some(Poll::Pending)             => { }
                Some(Poll::Ready(None))         => { follow_right = None; }
                Some(Poll::Ready(Some(action))) => {
                    // Send to the LHS 
                    for action in concatenator.send_right(iter::once(action)) {
                        pending.push_back(action);
                    }

                    // Return any pending actions
                    if let Some(next) = pending.pop_front() { return Poll::Ready(Some(next)); }
                }
            }

            // No actions: will be woken up once something happens to either of the two streams
            Poll::Pending
        });

        // Result is a rope reading from this stream
        RopeBinding::from_stream(concat_stream)
    }
} 
