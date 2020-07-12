use super::traits::*;
use super::releasable::*;

use flo_rope::*;
use ::desync::*;
use futures::task::*;
use futures::prelude::*;
use futures::future::{BoxFuture};

use std::mem;
use std::pin::*;
use std::sync::*;
use std::collections::{VecDeque};

///
/// The core of a rope binding represents the data that's shared amongst all ropes
///
struct RopeBindingCore<Cell, Attribute> 
where
Cell:       Clone+PartialEq,
Attribute:  Clone+PartialEq+Default {
    /// The number of items that are using hte core
    usage_count: usize,

    /// The rope that stores this binding
    rope: PullRope<AttributedRope<Cell, Attribute>, Box<dyn Fn() -> ()+Send+Sync>>,

    /// The states of any streams reading from this rope
    stream_states: Vec<RopeStreamState<Cell, Attribute>>,

    /// The next ID to assign to a stream state
    next_stream_id: usize,

    // List of things to call when this binding changes
    when_changed: Vec<ReleasableNotifiable>
}

///
/// The state of a stream that is reading from a rope binding core
///
struct RopeStreamState<Cell, Attribute>
where
Cell:       Clone+PartialEq,
Attribute:  Clone+PartialEq+Default {
    /// The identifier for this stream
    identifier: usize,

    /// The waker for the current stream
    waker: Option<Waker>,

    /// The changes that are waiting to be sent to this stream
    pending_changes: VecDeque<RopeAction<Cell, Attribute>>
}

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

///
/// A rope stream monitors a rope binding, and supplies them as a stream so they can be mirrored elsewhere
///
/// An example of a use for a rope stream is to send updates from a rope to a user interface.
///
pub struct RopeStream<Cell, Attribute> 
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    /// The identifier for this stream
    identifier: usize,

    /// The core of the rope
    core: Arc<Desync<RopeBindingCore<Cell, Attribute>>>,

    /// A future that will return the next poll result
    poll_future: Option<BoxFuture<'static, Poll<Option<VecDeque<RopeAction<Cell, Attribute>>>>>>,

    /// The actions that are currently being drained through this stream
    draining: VecDeque<RopeAction<Cell, Attribute>>,

    /// The notification that wakes up this stream
    notification: Box<dyn Releasable>
}

impl<Cell, Attribute> RopeBindingCore<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
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

    ///
    /// Pulls values from the rope and send to all attached streams
    ///
    fn pull_rope(&mut self) {
        // Collect the actions
        let actions = self.rope.pull_changes().collect::<Vec<_>>();

        // Push to each stream
        for stream in self.stream_states.iter_mut() {
            stream.pending_changes.extend(actions.iter().cloned());
        }

        // Wake all of the streams
        for stream in self.stream_states.iter_mut() {
            let waker = stream.waker.take();
            waker.map(|waker| waker.wake());
        }
    }

    ///
    /// Wakes a particular stream when the rope changes
    ///
    fn wake_stream(&mut self, stream_id: usize, waker: Waker) {
        self.stream_states
            .iter_mut()
            .filter(|state| state.identifier == stream_id)
            .nth(0)
            .map(move |state| state.waker = Some(waker));
    }
}

impl<Cell, Attribute> RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    ///
    /// Creates a new rope binding from a stream of changes
    ///
    pub fn from_stream<S: 'static+Stream<Item=RopeAction<Cell, Attribute>>+Unpin+Send>(stream: S) -> RopeBinding<Cell, Attribute> {
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

impl<Cell, Attribute> Stream for RopeStream<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+Unpin+PartialEq+Default {
    type Item = RopeAction<Cell,Attribute>;

    fn poll_next(mut self: Pin<&mut Self>, ctxt: &mut Context<'_>) -> Poll<Option<RopeAction<Cell, Attribute>>> { 
        // If we've got a set of actions we're already reading, then return those as fast as we can
        if self.draining.len() > 0 {
            return Poll::Ready(self.draining.pop_back());
        }

        // If we're waiting for the core to return to us, borrow the future from there
        let poll_future     = self.poll_future.take();
        let mut poll_future = if let Some(poll_future) = poll_future {
            // We're already waiting for the core to get back to us
            poll_future
        } else {
            // Ask the core for the next stream state
            let stream_id = self.identifier;

            self.core.future(move |core| {
                async move {
                    // Find the state of this stream
                    let stream_state = core.stream_states.iter_mut()
                        .filter(|state| state.identifier == stream_id)
                        .nth(0)
                        .unwrap();

                    // Check for data
                    if stream_state.pending_changes.len() > 0 {
                        // Return the changes to the waiting stream
                        let mut changes = VecDeque::new();
                        mem::swap(&mut changes, &mut stream_state.pending_changes);

                        Poll::Ready(Some(changes))
                    } else if core.usage_count == 0 {
                        // No changes, and nothing is using the core any more
                        Poll::Ready(None)
                    } else {
                        // No changes are waiting
                        Poll::Pending
                    }
                }.boxed()
            })
            .map(|result| {
                // Error would indicate the core had gone away before the request should complete, so we signal this as an end-of-stream event
                match result {
                    Ok(result)  => result,
                    Err(_)      => Poll::Ready(None)
                }
            })
            .boxed()
        };

        // Ask the future for the latest update on this stream
        let future_result = poll_future.poll_unpin(ctxt);

        match future_result {
            Poll::Ready(Poll::Ready(Some(actions))) => {
                if actions.len() == 0 {
                    // Nothing waiting: need to wait until the rope signals a 'pull' event
                    let waker       = ctxt.waker().clone();
                    let stream_id   = self.identifier;

                    self.core.desync(move |core| {
                        core.wake_stream(stream_id, waker);
                    });

                    Poll::Pending
                } else {
                    // Have some actions ready
                    self.draining = actions;
                    Poll::Ready(self.draining.pop_back())
                }
            }

            Poll::Ready(Poll::Ready(None))  => Poll::Ready(None),
            Poll::Ready(Poll::Pending)      => {
                // Wake when the rope generates a 'pull' event
                let waker       = ctxt.waker().clone();
                let stream_id   = self.identifier;

                self.core.desync(move |core| {
                    core.wake_stream(stream_id, waker);
                });

                Poll::Pending
            }

            Poll::Pending                   => {
                // Poll the future again when it notifies
                self.poll_future = Some(poll_future);
                Poll::Pending
            }
        }
    }
}
