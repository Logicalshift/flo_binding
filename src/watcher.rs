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
