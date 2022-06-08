//!
//! # `flo_binding`, a data-driven binding library
//!
//! `flo_binding` provides a way to create 'data bindings', which are a way to store
//! and share state between different parts of an application. Applications designed
//! around these data structures are sometimes referred to as 'reactive' applications.
//! 
//! A basic binding can be created using the `bind()` function:
//! 
//! ```
//! # use flo_binding::*;
//!     let binding = bind(1);
//! ```
//! 
//! This can be updated by calling `set()` and retrieved using `get()`:
//! 
//! ```
//! # use flo_binding::*;
//! # let binding = bind(1);
//!     let value = binding.get(); // == 1
//! # assert!(value == 1);
//!     binding.set(2);
//!     let value = binding.get(); // == 2
//! # assert!(value == 2);
//! ```
//! 
//! Cloning a binding will share the state:
//! 
//! ```
//! # use flo_binding::*;
//! # let binding = bind(1);
//!     let another_binding = binding.clone();
//!     another_binding.set(3);
//!     let value = another_binding.get();  // == 3
//! # assert!(value == 3);
//!     let value = binding.get();          // Also == 3
//! # assert!(value == 3);
//! ```
//! 
//! There are two main ways to be notified when a binding has changed. The 
//! `when_changed()` function provides a way to call a function whenever a binding
//! has changed since it was last read, and the `follow()` function will return a 
//! Stream of the most recent value attached to the binding. The streaming approach
//! is the most flexible.
//! 
//! ```
//! # use flo_binding::*;
//! # use futures::prelude::*;
//! # use futures::executor;
//! # executor::block_on(async {
//!     let binding             = bind(1);
//!     let mut event_lifetime  = binding.when_changed(notify(|| { println!("Binding changed"); }));
//!     let mut binding_stream  = follow(binding.clone());
//! 
//!     let value = binding_stream.next().await;        // == 1
//! # assert!(value == Some(1));
//!     binding.set(2);                                 // Prints 'Binding changed'
//!     binding.set(3);                                 // Changed flag is set, so prints nothing
//!     let value = binding.get();                      // == 3
//! # assert!(value == 3);
//! 
//!     binding.set(4);                                 // Prints 'Binding changed' again
//!     let value = binding_stream.next().await;        // == 4 (stream does not return intermediate values)
//! # assert!(value == Some(4));
//! # binding.set(5);
//!     let value = binding_stream.next().await;        // Blocks until something elsewhere updates the binding
//! # assert!(value == Some(5));
//! 
//!     event_lifetime.done();
//!     binding.set(5);                                 // Prints nothing as the 'when_changed' event has been deregistered
//! # });
//! ```
//! 
//! The inverse of `follow()` is `bind_stream()`, which creates a bindings that is kept
//! up to date with whatever the last value received from a stream is:
//! 
//! ```
//! # use flo_binding::*;
//!     let binding             = bind(1);
//!     let binding_from_stream = bind_stream(follow(binding.clone()), 1, |_old_value, new_value| new_value);
//! 
//!     let value = binding.get();                  // == 1
//!     let value = binding_from_stream.get();      // == 1
//! 
//!     binding.set(2);
//!     let value = binding_from_stream.get();      // == 2
//! ```
//! 
//! A stream binding like this is read-only, but is a good way to convert any stream of
//! state values into a value representing a static state. It implements all of the other
//! binding operations.
//! 
//! Another important type of binding is the `computed()` binding, which makes it possible 
//! to create a binding that derives a value from other bindings. Computed bindings 
//! automatically monitor any bindings that were captured for changes, so they can be
//! `follow`ed or `when_change`d as with any other binding:
//! 
//! ```
//! # use flo_binding::*;
//!     let binding         = bind(1);
//!     let binding_copy    = binding.clone();
//!     let one_more        = computed(move || binding_copy.get() + 1);
//! 
//!     let event_lifetime  = one_more.when_changed(notify(|| println!("Computed binding changed")));
//! 
//!     let value = one_more.get();     // == 2 (1 + 1)
//!     binding.set(2);                 // Prints 'Computed binding changed'
//!     binding.set(3);                 // Computed binding has not been read since the last notification so prints nothing
//!     let value = one_more.get();     // == 4 (3 + 1)
//! ```
//! 
//! For collections of data, `flo_binding` uses the concept of a 'rope binding'. The 
//! general rope data type is provided by the [`flo_rope`](https://crates.io/crates/flo_rope)
//! crate. These bindings send differences rather than their full state when streaming and
//! are internally represented by a data structure that can perform deletions and insertions
//! efficiently. Unlike the traditional concept of a rope, they aren't limited to editing
//! strings, and can annotate their contents with attributes, which makes them suitable for
//! representing sequences of any kind, or sequences of rich text annotated with style
//! information.
//! 
//! ```
//! # use flo_binding::*;
//! # use futures::prelude::*;
//! # use futures::executor;
//! # executor::block_on(async {
//!     let mutable_rope        = RopeBindingMut::<usize, ()>::new();
//!     let rope_copy           = RopeBinding::from_stream(mutable_rope.follow_changes());
//!     let mut rope_stream     = rope_copy.follow_changes();
//! 
//!     mutable_rope.replace(0..0, vec![1, 2, 3, 4]);
//! 
//!     let next            = rope_stream.next().await;                         // == RopeAction::Replace(0..0, vec![1,2,3,4]))
//! 
//!     let rope_len        = rope_copy.len();                                  // == 4
//!     let rope_content    = rope_copy.read_cells(0..4).collect::<Vec<_>>();   // == vec![1, 2, 3, 4]
//! # });
//! ```
//! 
//! The `flo_rope` library provides some extra functionality - for example, a way to create the
//! `RopeAction`s for a rope by using a diff algorithm on a changing sequence instead of just
//! reporting the changes as they arrive.
//! 
//! ## Companion libraries
//! 
//! Aside from `flo_rope`, the [`desync`](https://crates.io/crates/desync) crate provides a 
//! novel approach to asynchronous code, including pipe operations that work very well with 
//! the streamed updates from the `follow()` function.
//! 
//! The [`flo_stream`](https://crates.io/crates/flo_stream) provides a pubsub system that
//! provides more flexible ways for distributing state through streams.
//! 
//! [`flo_scene`](https://github.com/logicalshift/flo_scene) is a runtime for building
//! complex systems out of entities that exchange messages with each other. It uses
//! `flo_binding` as an ergonomic way to exchange state information.
//!

#![warn(bare_trait_objects)]

mod traits;
pub mod binding_context;
mod binding;
mod computed;
mod bindref;
mod notify_fn;
mod releasable;
#[cfg(feature = "stream")]
mod follow;
#[cfg(feature = "stream")]
mod bind_stream;
#[cfg(feature = "rope")]
mod rope_binding;

pub use self::traits::*;
pub use self::binding::*;
pub use self::computed::*;
pub use self::bindref::*;
pub use self::notify_fn::*;
#[cfg(feature = "stream")]
pub use self::follow::*;
#[cfg(feature = "stream")]
pub use self::bind_stream::*;
#[cfg(feature = "rope")]
pub use self::rope_binding::*;

///
/// Creates a simple bound value with the specified initial value
///
pub fn bind<Value: Clone+PartialEq>(val: Value) -> Binding<Value> {
    Binding::new(val)
}

///
/// Creates a computed value that tracks bindings accessed during the function call and marks itself as changed when any of these dependencies also change
///
pub fn computed<Value, TFn>(calculate_value: TFn) -> ComputedBinding<Value, TFn>
where Value: Clone+Send, TFn: 'static+Send+Sync+Fn() -> Value {
    ComputedBinding::new(calculate_value)
}

#[cfg(test)]
mod test {
    use super::*;
    use super::binding_context::*;

    use std::thread;
    use std::sync::*;
    use std::time::Duration;

    #[test]
    fn can_create_binding() {
        let bound = bind(1);
        assert!(bound.get() == 1);
    }

    #[test]
    fn can_update_binding() {
        let bound = bind(1);

        bound.set(2);
        assert!(bound.get() == 2);
    }

    #[test]
    fn notified_on_change() {
        let bound       = bind(1);
        let changed     = bind(false);

        let notify_changed = changed.clone();
        bound.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(changed.get() == false);
        bound.set(2);
        assert!(changed.get() == true);
    }

    #[test]
    fn not_notified_on_no_change() {
        let bound       = bind(1);
        let changed     = bind(false);

        let notify_changed = changed.clone();
        bound.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(changed.get() == false);
        bound.set(1);
        assert!(changed.get() == false);
    }

    #[test]
    fn notifies_after_each_change() {
        let bound           = bind(1);
        let change_count    = bind(0);

        let notify_count    = change_count.clone();
        bound.when_changed(notify(move || { let count = notify_count.get(); notify_count.set(count+1) })).keep_alive();

        assert!(change_count.get() == 0);
        bound.set(2);
        assert!(change_count.get() == 1);

        bound.set(3);
        assert!(change_count.get() == 2);

        bound.set(4);
        assert!(change_count.get() == 3);
    }

    #[test]
    fn dispatches_multiple_notifications() {
        let bound           = bind(1);
        let change_count    = bind(0);

        let notify_count    = change_count.clone();
        let notify_count2   = change_count.clone();
        bound.when_changed(notify(move || { let count = notify_count.get(); notify_count.set(count+1) })).keep_alive();
        bound.when_changed(notify(move || { let count = notify_count2.get(); notify_count2.set(count+1) })).keep_alive();

        assert!(change_count.get() == 0);
        bound.set(2);
        assert!(change_count.get() == 2);

        bound.set(3);
        assert!(change_count.get() == 4);

        bound.set(4);
        assert!(change_count.get() == 6);
    }

    #[test]
    fn stops_notifying_after_release() {
        let bound           = bind(1);
        let change_count    = bind(0);

        let notify_count = change_count.clone();
        let mut lifetime = bound.when_changed(notify(move || { let count = notify_count.get(); notify_count.set(count+1) }));

        assert!(change_count.get() == 0);
        bound.set(2);
        assert!(change_count.get() == 1);

        lifetime.done();
        assert!(change_count.get() == 1);
        bound.set(3);
        assert!(change_count.get() == 1);
    }

    #[test]
    fn release_only_affects_one_notification() {
        let bound           = bind(1);
        let change_count    = bind(0);

        let notify_count    = change_count.clone();
        let notify_count2   = change_count.clone();
        let mut lifetime    = bound.when_changed(notify(move || { let count = notify_count.get(); notify_count.set(count+1) }));
        bound.when_changed(notify(move || { let count = notify_count2.get(); notify_count2.set(count+1) })).keep_alive();

        assert!(change_count.get() == 0);
        bound.set(2);
        assert!(change_count.get() == 2);

        bound.set(3);
        assert!(change_count.get() == 4);

        bound.set(4);
        assert!(change_count.get() == 6);

        lifetime.done();

        bound.set(5);
        assert!(change_count.get() == 7);

        bound.set(6);
        assert!(change_count.get() == 8);

        bound.set(7);
        assert!(change_count.get() == 9);
    }

    #[test]
    fn binding_context_is_notified() {
        let bound = bind(1);

        bound.set(2);

        let (value, context) = BindingContext::bind(|| bound.get());
        assert!(value == 2);

        let changed = bind(false);
        let notify_changed = changed.clone();
        context.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(changed.get() == false);
        bound.set(3);
        assert!(changed.get() == true);
    }

    #[test]
    fn can_compute_value() {
        let bound           = bind(1);

        let computed_from   = bound.clone();
        let computed        = computed(move || computed_from.get() + 1);

        assert!(computed.get() == 2);
    }

    #[test]
    fn can_recompute_value() {
        let bound           = bind(1);

        let computed_from   = bound.clone();
        let computed        = computed(move || computed_from.get() + 1);

        assert!(computed.get() == 2);

        bound.set(2);
        assert!(computed.get() == 3);

        bound.set(3);
        assert!(computed.get() == 4);
    }

    #[test]
    fn can_recursively_compute_values() {
        let bound               = bind(1);

        let computed_from       = bound.clone();
        let computed_val        = computed(move || computed_from.get() + 1);

        let more_computed_from  = computed_val.clone();
        let more_computed       = computed(move || more_computed_from.get() + 1);

        assert!(computed_val.get() == 2);
        assert!(more_computed.get() == 3);

        bound.set(2);
        assert!(computed_val.get() == 3);
        assert!(more_computed.get() == 4);

        bound.set(3);
        assert!(computed_val.get() == 4);
        assert!(more_computed.get() == 5);
    }

    #[test]
    fn can_recursively_compute_values_2() {
        let bound               = bind(1);

        let computed_from       = bound.clone();
        let computed_val        = computed(move || computed_from.get() + 1);
        let more_computed       = computed(move || computed_val.get() + 1);

        assert!(more_computed.get() == 3);

        bound.set(2);
        assert!(more_computed.get() == 4);

        bound.set(3);
        assert!(more_computed.get() == 5);
    }

    #[test]
    fn can_recursively_compute_values_3() {
        let bound               = bind(1);

        let computed_from       = bound.clone();
        let computed_val        = computed(move || computed_from.get() + 1);
        let more_computed       = computed(move || computed_val.get() + 1);
        let even_more_computed  = computed(move || more_computed.get() + 1);

        assert!(even_more_computed.get() == 4);

        bound.set(2);
        assert!(even_more_computed.get() == 5);

        bound.set(3);
        assert!(even_more_computed.get() == 6);
    }

    #[test]
    #[should_panic]
    fn panics_if_computed_generated_during_binding() {
        let bound               = bind(1);

        let computed_from       = bound.clone();
        let computed_val        = computed(move || computed_from.get() + 1);
        let even_more_computed  = computed(move || {
            let computed_val = computed_val.clone();

            // This computed binding would be dropped after the first evaluation, which would result in the binding never updating.
            // We should panic here.
            let more_computed = computed(move || computed_val.get() + 1);
            more_computed.get() + 1
        });

        assert!(even_more_computed.get() == 4);

        bound.set(2);
        assert!(even_more_computed.get() == 5);

        bound.set(3);
        assert!(even_more_computed.get() == 6);
    }

    #[test]
    fn computed_only_recomputes_as_needed() {
        let bound               = bind(1);

        let counter             = Arc::new(Mutex::new(0));
        let compute_counter     = counter.clone();
        let computed_from       = bound.clone();
        let computed            = computed(move || {
            let mut counter = compute_counter.lock().unwrap();
            *counter = *counter + 1;

            computed_from.get() + 1
        });

        assert!(computed.get() == 2);
        {
            let counter = counter.lock().unwrap();
            assert!(counter.clone() == 1);
        }

        assert!(computed.get() == 2);
        {
            let counter = counter.lock().unwrap();
            assert!(counter.clone() == 1);
        }

        bound.set(2);
        assert!(computed.get() == 3);
        {
            let counter = counter.lock().unwrap();
            assert!(counter.clone() == 2);
        }
    }

    #[test]
    fn computed_caches_values() {
        let update_count            = Arc::new(Mutex::new(0));
        let bound                   = bind(1);

        let computed_update_count   = Arc::clone(&update_count);
        let computed_from           = bound.clone();
        let computed                = computed(move || {
            let mut computed_update_count = computed_update_count.lock().unwrap();
            *computed_update_count += 1;

            computed_from.get() + 1
        });

        assert!(computed.get() == 2);
        assert!(*update_count.lock().unwrap() == 1);

        assert!(computed.get() == 2);
        assert!(*update_count.lock().unwrap() == 1);

        bound.set(2);
        assert!(computed.get() == 3);
        assert!(*update_count.lock().unwrap() == 2);

        bound.set(3);
        assert!(*update_count.lock().unwrap() == 2);
        assert!(computed.get() == 4);
        assert!(*update_count.lock().unwrap() == 3);
    }

    #[test]
    fn computed_notifies_of_changes() {
        let bound           = bind(1);

        let computed_from   = bound.clone();
        let computed        = computed(move || computed_from.get() + 1);

        let changed         = bind(false);
        let notify_changed  = changed.clone();
        computed.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(computed.get() == 2);
        assert!(changed.get() == false);

        bound.set(2);
        assert!(changed.get() == true);
        assert!(computed.get() == 3);

        changed.set(false);
        bound.set(3);
        assert!(changed.get() == true);
        assert!(computed.get() == 4);
    }

    #[test]
    fn computed_switches_dependencies() {
        let switch          = bind(false);
        let val1            = bind(1);
        let val2            = bind(2);

        let computed_switch = switch.clone();
        let computed_val1   = val1.clone();
        let computed_val2   = val2.clone();
        let computed        = computed(move || {
            // Use val1 when switch is false, and val2 when switch is true
            if computed_switch.get() {
                computed_val2.get() + 1
            } else {
                computed_val1.get() + 1
            }
        });

        let changed         = bind(false);
        let notify_changed  = changed.clone();
        computed.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        // Initial value of computed (first get 'arms' when_changed too)
        assert!(computed.get() == 2);
        assert!(changed.get() == false);

        // Setting val2 shouldn't cause computed to become 'changed' initially
        val2.set(3);
        assert!(changed.get() == false);
        assert!(computed.get() == 2);

        // ... but setting val1 should
        val1.set(2);
        assert!(changed.get() == true);
        assert!(computed.get() == 3);

        // Flicking the switch will use the val2 value we set earlier
        changed.set(false);
        switch.set(true);
        assert!(changed.get() == true);
        assert!(computed.get() == 4);

        // Updating val2 should now mark us as changed
        changed.set(false);
        val2.set(4);
        assert!(changed.get() == true);
        assert!(computed.get() == 5);
        
        // Updating val1 should not mark us as changed
        changed.set(false);
        val1.set(5);
        assert!(changed.get() == false);
        assert!(computed.get() == 5);
    }

    #[test]
    fn change_during_computation_recomputes() {
        // Create a computed binding that delays for a bit while reading
        let some_binding = bind(1);
        let some_computed = {
            let some_binding = some_binding.clone();
            computed(move || {
                let result = some_binding.get() + 1;
                thread::sleep(Duration::from_millis(250));
                result
            })
        };

        // Start a thread that reads a value
        {
            let some_computed = some_computed.clone();
            thread::spawn(move || {
                assert!(some_computed.get() == 2);
            });
        }

        // Let the thread start running (give it enough time to start computing and reach the sleep statement)
        // TODO: thread::sleep might fail on systems that are slow enough or due to glitches (will fail spuriously if we update the binding before the calculation starts)
        thread::sleep(Duration::from_millis(10));

        // Update the value in the binding while the computed is running
        some_binding.set(2);

        // Computed value should update
        assert!(some_computed.get() == 3);
    }

    #[test]
    fn computed_propagates_changes() {
        let bound               = bind(1);

        let computed_from       = bound.clone();
        let propagates_from     = computed(move || computed_from.get() + 1);
        let computed_propagated = propagates_from.clone();
        let computed            = computed(move || computed_propagated.get() + 1);

        let changed             = bind(false);
        let notify_changed      = changed.clone();
        computed.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(propagates_from.get() == 2);
        assert!(computed.get() == 3);
        assert!(changed.get() == false);

        bound.set(2);
        assert!(propagates_from.get() == 3);
        assert!(computed.get() == 4);
        assert!(changed.get() == true);

        changed.set(false);
        bound.set(3);
        assert!(changed.get() == true);
        assert!(propagates_from.get() == 4);
        assert!(computed.get() == 5);
    }

    #[test]
    fn computed_stops_notifying_when_released() {
        let bound           = bind(1);

        let computed_from   = bound.clone();
        let computed        = computed(move || computed_from.get() + 1);

        let changed         = bind(false);
        let notify_changed  = changed.clone();
        let mut lifetime    = computed.when_changed(notify(move || notify_changed.set(true)));

        assert!(computed.get() == 2);
        assert!(changed.get() == false);

        bound.set(2);
        assert!(changed.get() == true);
        assert!(computed.get() == 3);

        changed.set(false);
        lifetime.done();

        bound.set(3);
        assert!(changed.get() == false);
        assert!(computed.get() == 4);

        bound.set(4);
        assert!(changed.get() == false);
        assert!(computed.get() == 5);
    }

    #[test]
    fn computed_doesnt_notify_more_than_once() {
        let bound           = bind(1);

        let computed_from   = bound.clone();
        let computed        = computed(move || computed_from.get() + 1);

        let changed         = bind(false);
        let notify_changed  = changed.clone();
        computed.when_changed(notify(move || notify_changed.set(true))).keep_alive();

        assert!(computed.get() == 2);
        assert!(changed.get() == false);

        // Setting the value marks the computed as changed
        bound.set(2);
        assert!(changed.get() == true);
        changed.set(false);

        // ... but when it's already changed we don't notify again
        bound.set(3);
        assert!(changed.get() == false);

        assert!(computed.get() == 4);

        // Once we've retrieved the value, we'll get notified of changes again
        bound.set(4);
        assert!(changed.get() == true);
    }

    #[test]
    fn computed_stops_notifying_once_out_of_scope() {
        let bound           = bind(1);
        let changed         = bind(false);

        {
            let computed_from   = bound.clone();
            let computed        = computed(move || computed_from.get() + 1);

            let notify_changed  = changed.clone();
            computed.when_changed(notify(move || notify_changed.set(true))).keep_alive();

            assert!(computed.get() == 2);
            assert!(changed.get() == false);

            bound.set(2);
            assert!(changed.get() == true);
            assert!(computed.get() == 3);
        };

        // The computed value should have been disposed of so we should get no more notifications once we reach here
        changed.set(false);
        bound.set(3);
        assert!(changed.get() == false);
    }
}
