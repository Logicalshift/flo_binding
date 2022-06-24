use crate::watcher::*;

use std::sync::*;

///
/// Trait implemented by items with dependencies that need to be notified when they have changed
///
pub trait Notifiable : Sync+Send {
    ///
    /// Indicates that a dependency of this object has changed
    ///
    fn mark_as_changed(&self);
}

///
/// Trait implemented by an object that can be released: for example to stop performing
/// an action when it's no longer required.
///
pub trait Releasable : Send {
    ///
    /// Indicates that this object should not be released on drop
    ///
    fn keep_alive(&mut self);

    ///
    /// Indicates that this object is finished with and should be released
    ///
    fn done(&mut self);
}

///
/// Trait implemented by items that can notify something when they're changed
///
pub trait Changeable {
    ///
    /// Supplies a function to be notified when this item is changed
    ///
    /// This will always fire if the value has been changed since it was last 
    /// read. The notification may fire more often than this depending on the
    /// implementation of the `Changeable` trait.
    /// 
    /// The releasable that's returned has keep_alive turned off by default, so
    /// be sure to store it in a variable or call keep_alive() to keep it around
    /// (if the event never seems to fire, this is likely to be the problem)
    ///
    fn when_changed(&self, what: Arc<dyn Notifiable>) -> Box<dyn Releasable>;
}

///
/// Trait implemented by something that is bound to a value
///
pub trait Bound<Value> : Changeable + Send + Sync {
    ///
    /// Retrieves the value stored by this binding
    ///
    fn get(&self) -> Value;

    ///
    /// Creates a watcher: this provides a way to retrieve the value stored in this 
    /// binding, and will call the notification function if the value has changed 
    /// since it was last read.
    ///
    /// This is a non-async version of the `follow()` function.
    ///
    fn watch(&self, what: Arc<dyn Notifiable>) -> Arc<dyn Watcher<Value>>;
}

///
/// Trait implemented by something that is bound to a value
///
// Seperate Trait to allow Bound to be made into an object for BindRef
pub trait WithBound<Value>: Changeable + Send + Sync {
    ///
    /// Mutate instead of replacing value stored in this binding, return true
    /// to send notifiations
    ///
    fn with_ref<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Value) -> T;
    ///
    /// Mutate instead of replacing value stored in this binding, return true
    /// to send notifiations
    ///
    fn with_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut Value) -> bool;
}

///
/// Trait implemented by something that is bound to a value that can be changed
/// 
/// Bindings are similar in behaviour to Arc<Mutex<Value>>, so it's possible to set 
/// the value of their target even when the binding itself is not mutable.
///
pub trait MutableBound<Value> : Bound<Value> {
    ///
    /// Sets the value stored by this binding
    ///
    fn set(&self, new_value: Value);
}
