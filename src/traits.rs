use crate::bindref::*;
use crate::map_binding::*;

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
pub trait Bound : Changeable + Send + Sync {
    type Value;

    ///
    /// Retrieves the value stored by this binding
    ///
    fn get(&self) -> Self::Value;

    ///
    /// Creates a watcher: this provides a way to retrieve the value stored in this 
    /// binding, and will call the notification function if the value has changed 
    /// since it was last read.
    ///
    /// This is a non-async version of the `follow()` function.
    ///
    fn watch(&self, what: Arc<dyn Notifiable>) -> Arc<dyn Watcher<Self::Value>>;
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
pub trait MutableBound : Bound {
    ///
    /// Sets the value stored by this binding
    ///
    fn set(&self, new_value: Self::Value);
}

///
/// A watcher provides a way to access a value referenced by a binding. It is associated
/// with a notification, which is fired if the value has been changed since the last call
/// to the `get()` function for this Watcher.
///
/// This means that `get()` must be called at least once for the watcher for the notification
/// to fire, and that the notification will not fire if the binding is read by any other
/// part of the application.
///
/// The notification will no longer be fired if the watcher is disposed.
///
pub trait Watcher<TValue> {
    ///
    /// Reads the current value of the binding. The notification associated with this watcher
    /// will be fired if the value is changed from the last value that was returned by this
    /// call.
    ///
    fn get(&self) -> TValue;
}

///
/// Provides the `compute()` function for bindings and compatible types
///
pub trait BoundValueComputeExt : Sized {
    /// 
    /// Transforms the value of this binding using a compute function
    ///
    /// This is generally used as a convenience function. It's often necessary to create copies of the
    /// bindings and move them into the closure for `computed()`, which can add a lot of extra code and
    /// requires some sort of naming convention to distinguish the originals and the copies.
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding        = bind(1);
    /// let also_some_binding   = some_binding.clone();
    /// let mapped              = computed(move || also_some_binding.get() + 1);
    /// ```
    ///
    /// The clone can be avoided using the `compute()` function:
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding    = bind(1);
    /// let mapped          = some_binding.compute(|val| val.get() + 1);
    /// ```
    ///
    /// This is supported on tuples of bindings too:
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding    = bind(1);
    /// let another_binding = bind(2);
    /// let mapped          = (some_binding, another_binding)
    ///     .compute(|(some_binding, another_binding)| some_binding.get() + another_binding.get());
    /// ```
    ///
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue;
}

///
/// Provides the map function for bindings
///
pub trait BoundValueMapExt {
    type Value;

    /// 
    /// Transforms the value of this binding using a mapping function
    ///
    /// This will track other bindings used in the mapping like `computed()` does: that is, this:
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding        = bind(1);
    /// let also_some_binding   = some_binding.clone();
    /// let mapped              = computed(move || also_some_binding.get() + 1);
    /// ```
    ///
    /// Could also be written like this using this `map()` function:
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding    = bind(1);
    /// let mapped          = some_binding.map_binding(|val| val + 1);
    /// ```
    ///
    /// There's also a way to do this using an async stream binding, if flo_binding is compiled with
    /// the stream feature enabled:
    ///
    /// ```
    /// # use flo_binding::*;
    /// let some_binding    = bind(1);
    /// let mapped          = bind_stream(follow(some_binding.clone()), 1, |_last_val, val| val + 1);
    /// ```
    ///
    fn map_binding<TMapValue, TMapFn>(&self, map_fn: TMapFn) -> MapBinding<Self, TMapValue, TMapFn>
    where
        TMapValue:  'static + Clone + Send,
        TMapFn:     'static + Send + Sync + Fn(Self::Value) -> TMapValue;
}
