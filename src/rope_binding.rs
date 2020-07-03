use super::traits::*;
use super::releasable::*;

use flo_rope::*;
use ::desync::*;

use std::sync::*;

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
    /// The rope used as stoage for this binding
    rope: Desync<PullRope<AttributedRope<Cell, Attribute>, Box<dyn Fn() -> ()+Send+Sync>>>,

    // List of things to call when this binding changes
    when_changed: Mutex<Vec<ReleasableNotifiable>>
}

impl<Cell, Attribute> RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
    ///
    /// If there are any notifiables in this object that aren't in use, remove them
    ///
    fn filter_unused_notifications(&self) {
        self.when_changed.lock().unwrap().retain(|releasable| releasable.is_in_use());
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
        let releasable = ReleasableNotifiable::new(what);
        self.when_changed.lock().unwrap().push(releasable.clone_as_owned());

        self.filter_unused_notifications();

        Box::new(releasable)
    }
}


///
/// Trait implemented by something that is bound to a value
///
impl<Cell, Attribute> Bound<Vec<Cell>> for RopeBinding<Cell, Attribute>
where 
Cell:       'static+Send+Unpin+Clone+PartialEq,
Attribute:  'static+Send+Sync+Clone+PartialEq+Default {
    ///
    /// Retrieves the value stored by this binding
    ///
    fn get(&self) -> Vec<Cell> {
        self.rope.sync(|rope| {
            rope.read_cells(0..rope.len()).cloned().collect()
        })
    }
}
