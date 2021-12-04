use crate::traits::*;
use crate::notify_fn::*;
use crate::releasable::*;
use crate::binding_context::*;
use crate::rope_binding::core::*;
use crate::rope_binding::stream::*;
use crate::rope_binding::bound_rope::*;
use crate::rope_binding::stream_state::*;
use crate::rope_binding::rope_binding_mut::*;

use flo_rope::*;
use ::desync::*;
use futures::prelude::*;
use futures::stream;
use futures::task::{Poll};

use std::mem;
use std::sync::*;
use std::ops::{Range};
use std::collections::{VecDeque};

///
/// A rope binding binds a vector of cells and attributes
///
/// It's also possible to use a normal `Binding<Vec<_>>` for this purpose. A rope binding has a
/// couple of advantages though: it can handle very large collections of items and it can notify
/// only the relevant changes instead of always notifying the entire structure.
///
/// Rope bindings are ideal for representing text areas in user interfaces, but can be used for
/// any collection data structure.
///
pub struct RopeBinding<Cell, Attribute> 
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Unpin+Clone+PartialEq+Default {
    /// The core of this binding
    core: Arc<Desync<RopeBindingCore<Cell, Attribute>>>,
}

impl<Cell, Attribute> RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    ///
    /// Generates a rope binding by tracking a mutable binding
    ///
    pub fn from_mutable(binding: &RopeBindingMut<Cell, Attribute>) -> Self {
        Self::from_stream(binding.follow_changes())
    }

    ///
    /// Creates a new rope binding from a stream of changes
    ///
    pub fn from_stream<S: 'static+Stream<Item=RopeAction<Cell, Attribute>>+Unpin+Send>(stream: S) -> Self {
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

        // Push changes through to the core rope from the stream
        pipe_in(Arc::clone(&core), stream, |core, actions| {
            async move {
                core.rope.edit(actions);
            }.boxed()
        });

        // Create the binding
        RopeBinding {
            core
        }
    }

    ///
    /// Creates a rope binding that entirely replaces its set of cells by following a computed value (the attributes will always
    /// have their default values when using this method)
    ///
    pub fn computed<TFn: 'static+Send+Fn() -> TValueIter, TValueIter: IntoIterator<Item=Cell>>(calculate_value: TFn) -> Self {
        // Create a stream of changes by following the function
        let mut length          = 0;
        let new_value           = Arc::new(Mutex::new(true));
        let waker               = Arc::new(Mutex::new(None));
        let dependency_monitor  = Arc::new(Mutex::new(None));

        let stream              = stream::poll_fn(move |ctxt| {
            // Store the waker so we can poll the stream again when it changes
            (*waker.lock().unwrap()) = Some(ctxt.waker().clone());

            // Replace the contents of the rope whenever there is a new value
            if mem::take(&mut (*new_value.lock().unwrap())) {
                // Loop until the value is stable
                loop {
                    // Release the monitor (this holds on to the bindings from the previous calculation)
                    (*dependency_monitor.lock().unwrap()) = None;

                    // Compute the new value and the dependencies
                    let (value_iter, dependencies)  = BindingContext::bind(|| calculate_value());

                    // When the dependencies change, mark that we've changed and wake up the stream
                    let new_value                   = Arc::clone(&new_value);
                    let waker                       = Arc::clone(&waker);
                    let new_dependency_monitor      = dependencies.when_changed_if_unchanged(notify(move || {
                        // Mark as changed
                        (*new_value.lock().unwrap()) = true;

                        // Wake the stream
                        let waker           = mem::take(&mut *waker.lock().unwrap());
                        if let Some(waker)  = waker { waker.wake() }
                    }));

                    // Recalculate the value if it has already changed
                    if new_dependency_monitor.is_none() { continue; }

                    // Keep the releasable alongside this stream
                    (*dependency_monitor.lock().unwrap()) = new_dependency_monitor;

                    // Return the new value
                    let new_cells   = value_iter.into_iter().collect::<Vec<_>>();
                    let old_length  = length;
                    length          = new_cells.len();

                    return Poll::Ready(Some(RopeAction::Replace(0..old_length, new_cells)));
                }
            } else {
                Poll::Pending
            }
        });

        Self::from_stream(stream)
    }

    ///
    /// Returns the number of cells in this rope
    ///
    pub fn len(&self) -> usize {
        self.core.sync(|core| core.rope.len())
    }

    ///
    /// Reads the cell values for a range in this rope
    ///
    pub fn read_cells<'a>(&'a self, range: Range<usize>) -> impl 'a+Iterator<Item=Cell> {
        // Read this range of cells by cloning from the core
        let cells = self.core.sync(|core| core.rope.read_cells(range).cloned().collect::<Vec<_>>());

        cells.into_iter()
    }

    ///
    /// Returns the attributes set at the specified location and their extent
    ///
    pub fn read_attributes<'a>(&'a self, pos: usize) -> (Attribute, Range<usize>) {
        let (attribute, range) = self.core.sync(|core| {
            let (attribute, range) = core.rope.read_attributes(pos);
            (attribute.clone(), range)
        });

        (attribute, range)
    }
}

impl<Cell, Attribute> BoundRope<Cell, Attribute> for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
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
                needs_pull:         false
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
        }
    }
}

impl<Cell, Attribute> Clone for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    fn clone(&self) -> RopeBinding<Cell, Attribute> {
        // Increase the usage count
        let core = self.core.clone();
        core.desync(|core| core.usage_count += 1);

        // Create a new binding with the same core
        RopeBinding { core }
    }
}

impl<Cell, Attribute> Drop for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
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

impl<Cell, Attribute> Changeable for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
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
impl<Cell, Attribute> Bound<AttributedRope<Cell, Attribute>> for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    ///
    /// Retrieves the value stored by this binding
    ///
    fn get(&self) -> AttributedRope<Cell, Attribute> {
        self.core.sync(|core| {
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
