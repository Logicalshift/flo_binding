use super::traits::*;
use super::releasable::*;

use flo_rope::*;
use flo_stream::*;
use ::desync::*;
use futures::prelude::*;

use std::sync::*;

///
/// The core of a rope binding represents the data that's shared amongst all ropes
///
struct RopeBindingCore<Cell, Attribute> 
where
Cell:       Clone+PartialEq,
Attribute:  Clone+PartialEq+Default {
    /// The rope that stores this binding
    rope: PullRope<AttributedRope<Cell, Attribute>, Box<dyn Fn() -> ()+Send+Sync>>,

    // List of things to call when this binding changes
    when_changed: Vec<ReleasableNotifiable>
}

///
/// A rope binding binds a vector of cells and attributes
///
/// It's also possible to use a normal `Binding<Vec<_>>` for this purpose. A rope binding has a
/// couple of advantages though: it can handle very large collections of items and it can notify
/// only the relevant changes instead of always notifying the entire structure.
///
pub struct RopeBinding<Cell, Attribute> 
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
    /// The core of this binding
    core: Arc<Desync<RopeBindingCore<Cell, Attribute>>>
}

impl<Cell, Attribute> RopeBindingCore<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
    ///
    /// If there are any notifiables in this object that aren't in use, remove them
    ///
    fn filter_unused_notifications(&mut self) {
        self.when_changed.retain(|releasable| releasable.is_in_use());
    }

    ///
    /// Callback: the rope has changes to pull
    ///
    fn on_pull(&mut self) {
        self.filter_unused_notifications();

        // Notify anything that's listening
        for notifiable in &self.when_changed {
            notifiable.mark_as_changed();
        }
    }
}

impl<Cell, Attribute> RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
    ///
    /// Creates a new rope binding from a stream of changes
    ///
    pub fn from_stream<S: 'static+Stream<Item=RopeAction<Cell, Attribute>>+Unpin+Send>(stream: S) -> RopeBinding<Cell, Attribute> {
        // Create the core
        let core        = RopeBindingCore {
            rope:           PullRope::from(AttributedRope::new(), Box::new(|| { })),
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
}

impl<Cell, Attribute> Changeable for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
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
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
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
