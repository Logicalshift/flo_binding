use crate::traits::*;
use crate::releasable::*;
use crate::binding_context::*;
use crate::rope_binding::core::*;
use crate::rope_binding::stream::*;
use crate::rope_binding::bound_rope::*;
use crate::rope_binding::stream_state::*;

use flo_rope::*;
use ::desync::*;

use std::sync::*;
use std::ops::{AddAssign, Range};
use std::collections::{VecDeque};
use std::iter;

///
/// A rope binding binds a vector of cells and attributes
///
/// A `RopeBindingMut` supplies the same functionality as a `RopeBinding` except it also provides the
/// editing functions for changing the underlying data.
///
pub struct RopeBindingMut<Cell, Attribute> 
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    /// The core of this binding
    core: Arc<Desync<RopeBindingCore<Cell, Attribute>>>,
}

impl<Cell, Attribute> RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    ///
    /// Creates a new rope binding from a stream of changes
    ///
    pub fn new() -> RopeBindingMut<Cell, Attribute> {
        // Create the core
        let core        = RopeBindingCore {
            usage_count:    1,
            rope:           PullRope::from(AttributedRope::new(), Box::new(|| { })),
            stream_states:  vec![],
            next_stream_id: 0,   
            when_changed:   vec![]
        };

        let core        = Arc::new(Desync::new(core));

        // Recreate the rope in the core with a version that responds to pull events
        let weak_core   = Arc::downgrade(&core);
        core.sync(move |core| {
            core.rope = PullRope::from(AttributedRope::new(), Box::new(move || {
                // Pass the event through to the core
                let core = weak_core.upgrade();
                if let Some(core) = core {
                    core.desync(|core| core.on_pull());
                }
            }));
        });

        // Create the binding
        RopeBindingMut {
            core
        }
    }

    ///
    /// Returns the number of cells in this rope
    ///
    pub fn len(&self) -> usize {
        BindingContext::add_dependency(self.clone());

        self.core.sync(|core| {
            core.pull_rope();
            core.rope.len()
        })
    }

    ///
    /// Reads the cell values for a range in this rope
    ///
    pub fn read_cells<'a>(&'a self, range: Range<usize>) -> impl 'a+Iterator<Item=Cell> {
        BindingContext::add_dependency(self.clone());

        // Read this range of cells by cloning from the core
        let cells = self.core.sync(|core| {
            core.pull_rope();
            core.rope.read_cells(range).cloned().collect::<Vec<_>>()
        });

        cells.into_iter()
    }

    ///
    /// Returns the attributes set at the specified location and their extent
    ///
    pub fn read_attributes<'a>(&'a self, pos: usize) -> (Attribute, Range<usize>) {
        BindingContext::add_dependency(self.clone());

        let (attribute, range) = self.core.sync(|core| {
            core.pull_rope();

            let (attribute, range) = core.rope.read_attributes(pos);
            (attribute.clone(), range)
        });

        (attribute, range)
    }

    /// 
    /// Performs the specified editing action to this rope
    ///
    pub fn edit(&self, action: RopeAction<Cell, Attribute>) {
        self.core.sync(move |core| {
            core.rope.edit(action);
            core.wake();
        });
    }

    ///
    /// Replaces a range of cells. The attributes applied to the new cells will be the same
    /// as the attributes that were applied to the first cell in the replacement range
    ///
    pub fn replace<NewCells>(&self, range: Range<usize>, new_cells: NewCells) 
    where
        NewCells: 'static + Send + IntoIterator<Item=Cell>,
    {
        self.core.sync(move |core| {
            core.rope.replace(range, new_cells);
            core.wake();
        });
    }

    ///
    /// Adds new cells to the end of this rope
    ///
    pub fn extend<I>(&self, iter: I)
    where
        I: 'static + Send + IntoIterator<Item=Cell>
    {
        let len = self.len();
        self.replace(len..len, iter);
    }

    ///
    /// Retains the cells that match the predicate
    ///
    pub fn retain_cells<TFn>(&self, retain_fn: TFn)
    where
        TFn: Send + FnMut(&Cell) -> bool,
    {
        self.core.sync(move |core| {
            // Find ranges where the function is false
            let mut start_pos       = 0;
            let mut last_result     = true;
            let mut replace_ranges  = vec![];
            let mut retain_fn       = retain_fn;

            for (idx, cell) in core.rope.read_cells(0..core.rope.len()).enumerate() {
                // See if we retain this cell
                let this_result = retain_fn(cell);

                if last_result == true && this_result == false {
                    // Moving from true -> false = update the start of the range
                    start_pos = idx;
                }

                if last_result == false && this_result == true {
                    // Moving from false -> true = remove this range
                    replace_ranges.push(start_pos..idx);
                }

                last_result = this_result;
            }

            if last_result == false {
                replace_ranges.push(start_pos..core.rope.len());
            }

            // Replace the ranges in the rope that are not retained with nothing
            if !replace_ranges.is_empty() {
                // Remove the cells from the rope
                replace_ranges.into_iter()
                    .rev()
                    .for_each(|range| core.rope.replace(range, iter::empty()));

                // Wake the core as there are changes
                core.wake();
            }
        })
    }

    ///
    /// Sets the attributes for a range of cells
    ///
    pub fn set_attributes(&self, range: Range<usize>, new_attributes: Attribute) {
        self.core.sync(move |core| {
            core.rope.set_attributes(range, new_attributes);
            core.wake();
        });
    }

    ///
    /// Replaces a range of cells and sets the attributes for them.
    ///
    pub fn replace_attributes<NewCells: 'static+Send+IntoIterator<Item=Cell>>(&self, range: Range<usize>, new_cells: NewCells, new_attributes: Attribute) {
        self.core.sync(move |core| {
            core.rope.replace_attributes(range, new_cells, new_attributes); 
            core.wake();
        });
    }
}

impl<Cell, Attribute> BoundRope<Cell, Attribute> for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    ///
    /// Creates a stream that follows the changes to this rope
    ///
    fn follow_changes(&self) -> RopeStream<Cell, Attribute> {
        // Fetch an ID for the next stream from the core and generate a state
        let stream_id = self.core.sync(|core| {
            // Assign an ID to the stream
            let next_id = core.next_stream_id;
            core.next_stream_id += 1;

            // Create a state for this stream
            let state = RopeStreamState {
                identifier:         next_id,
                waker:              None,
                pending_changes:    VecDeque::new(),
                needs_pull:         false,
            };
            core.stream_states.push(state);

            // Return the stream ID
            next_id
        });

        // Create the stream
        RopeStream {
            identifier:     stream_id,
            core:           self.core.clone(),
            poll_future:    None,
            draining:       VecDeque::new(),
            retains_core:   false,
        }
    }

    ///
    /// Creates a stream that follows the changes to this rope
    ///
    /// The stream will continue even if the rope binding is dropped (this is possible for RopeBinding as RopeBinding itself might be following a
    /// stream)
    ///
    fn follow_changes_retained(&self) -> RopeStream<Cell, Attribute> {
        // Mutable ropes can't continue to receive changes after they've been dropped so this still stops the stream once all copies of this rope are gone
        self.follow_changes()
    }
}

impl<Cell, Attribute> Clone for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin+PartialEq + Default,
{
    fn clone(&self) -> RopeBindingMut<Cell, Attribute> {
        // Increase the usage count
        let core = self.core.clone();
        core.desync(|core| core.usage_count += 1);

        // Create a new binding with the same core
        RopeBindingMut { core }
    }
}

impl<Cell, Attribute> Drop for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    fn drop(&mut self) {
        self.core.desync(|core| {
            // Core is no longer in use
            core.usage_count -= 1;

            // Counts as a notification if this is the last binding using this core
            if core.usage_count == 0 {
                core.pull_rope();
            }
        })
    }
}

impl<Cell, Attribute> Changeable for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    ///
    /// Supplies a function to be notified when this item is changed
    /// 
    /// This event is only fired after the value has been read since the most recent
    /// change. Note that this means if the value is never read, this event may
    /// never fire. This behaviour is desirable when deferring updates as it prevents
    /// large cascades of 'changed' events occurring for complicated dependency trees.
    /// 
    /// The releasable that's returned has keep_alive turned off by default, so
    /// be sure to store it in a variable or call keep_alive() to keep it around
    /// (if the event never seems to fire, this is likely to be the problem)
    ///
    fn when_changed(&self, what: Arc<dyn Notifiable>) -> Box<dyn Releasable> {
        let releasable      = ReleasableNotifiable::new(what);
        let core_releasable = releasable.clone_as_owned();

        self.core.desync(move |core| {
            core.when_changed.push(core_releasable);
            core.filter_unused_notifications();
        });

        Box::new(releasable)
    }
}


///
/// Trait implemented by something that is bound to a value
///
impl<Cell, Attribute> Bound<AttributedRope<Cell, Attribute>> for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    ///
    /// Retrieves the value stored by this binding
    ///
    fn get(&self) -> AttributedRope<Cell, Attribute> {
        BindingContext::add_dependency(self.clone());

        self.core.sync(|core| {
            core.pull_rope();

            // Create a new rope from the existing one
            let mut rope_copy   = AttributedRope::new();

            // Copy each attribute block one at a time
            let len             = core.rope.len();
            let mut pos         = 0;

            while pos < len {
                // Read the next range of attributes
                let (attr, range)   = core.rope.read_attributes(pos);
                if range.len() == 0 {
                    pos += 1;
                    continue;
                }

                // Write to the copy
                let attr    = attr.clone();
                let cells   = core.rope.read_cells(range.clone()).cloned();
                rope_copy.replace_attributes(pos..pos, cells, attr);

                // Continue writing at the end of the new rope
                pos         = range.end;
            }

            rope_copy
        })
    }
}

impl<Cell, Attribute> AddAssign<Cell> for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    fn add_assign(&mut self, other: Cell) {
        let len = self.len();
        self.replace(len..len, iter::once(other));
    }
}

impl<Cell, Attribute> PartialEq for RopeBindingMut<Cell, Attribute>
where 
    Cell:       'static + Send + Unpin + Clone + PartialEq,
    Attribute:  'static + Send + Sync + Clone + Unpin + PartialEq + Default,
{
    fn eq(&self, other: &RopeBindingMut<Cell, Attribute>) -> bool {
        // Compare lengths
        if self.len() != other.len() {
            return false;
        }

        // Compare cells
        let mut cells_a = self.read_cells(0..self.len());
        let mut cells_b = other.read_cells(0..other.len());

        while let (Some(a), Some(b)) = (cells_a.next(), cells_b.next()) {
            if a != b {
                return false;
            }
        }

        // Compare attributes (coverage of each attribute may vary between the two styles)
        let len                         = self.len();
        let (mut attr_a, mut range_a)   = self.read_attributes(0);
        let (mut attr_b, mut range_b)   = other.read_attributes(0);

        loop {
            // Ranges should never be 0-length
            debug_assert!(range_a.len() != 0 && range_b.len() != 0);

            // Move the 'a' range on if range_b starts after it, similarly for range_b
            if range_b.start >= range_a.end || range_a.start == range_a.end {
                let (new_attr, new_range) = self.read_attributes(range_b.start);
                attr_a  = new_attr;
                range_a = new_range;
            }

            if range_a.start >= range_b.end || range_b.start == range_b.end {
                let (new_attr, new_range) = other.read_attributes(range_a.start);
                attr_b  = new_attr;
                range_b = new_range;
            }

            // range_a and range_b should now overlap
            if attr_a != attr_b {
                return false;
            }

            // Move range_a or range_b onwards
            if range_a.end >= len && range_b.end >= len {
                // All attributes compared
                break;
            }

            if range_a.end <= range_b.end {
                let (new_attr, new_range) = self.read_attributes(range_a.end);
                attr_a  = new_attr;
                range_a = new_range;
            } else {
                let (new_attr, new_range) = other.read_attributes(range_b.end);
                attr_b  = new_attr;
                range_b = new_range;
            }
        }

        // All equality tests passed
        return true;
    }
}
