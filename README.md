
# `flo_binding`, a data-driven binding library

`flo_binding` provides a way to create 'data bindings', which are a way to store
and share state between different parts of an application. Applications designed
around these data structures are sometimes referred to as 'reactive' applications.

A basic binding can be created using the `bind()` function:

```Rust
    let binding = bind(1);
```

This can be updated by calling `set()` and retrieved using `get()`:

```Rust
    let value = binding.get(); // == 1
    binding.set(2);
    let value = binding.get(); // == 2
```

Cloning a binding will share the state:

```Rust
    let another_binding = binding.clone();
    another_binding.set(3);
    let value = another_binding.get();  // == 3
    let value = binding.get();          // Also == 3
```

There are two main ways to be notified when a binding has changed. The 
`when_changed()` function provides a way to call a function whenever a binding
has changed since it was last read, and the `follow()` function will return a 
Stream of the most recent value attached to the binding. The streaming approach
is the most flexible.

```Rust
    let binding         = bind(1);
    let event_lifetime  = binding.when_changed(notify(|| { println!("Binding changed"); }));
    let binding_stream  = follow(binding);

    let value = binding_stream.next().await;        // == 1
    binding.set(2);                                 // Prints 'Binding changed'
    binding.set(3);                                 // Changed flag is set, so prints nothing
    let value = binding.get();                      // == 3

    binding.set(4);                                 // Prints 'Binding changed' again
    let value = binding_stream.next().await;        // == 4 (stream does not return intermediate values)
    let value = binding_stream.next().await;        // Blocks until something elsewhere updates the binding

    event_lifetime.done();
    binding.set(5);                                 // Prints nothing as the 'when_changed' event has been deregistered
```

The inverse of `follow()` is `bind_stream()`, which creates a bindings that is kept
up to date with whatever the last value received from a stream is:

```Rust
    let binding             = bind(1);
    let binding_from_stream = bind_stream(follow(binding));

    let value = binding.get();                  // == 1
    let value = binding_from_stream.get();      // == 1

    binding.set(2);
    let value = binding_from_stream.get();      // == 2
```

A stream binding like this is read-only, but is a good way to convert any stream of
state values into a value representing a static state. It implements all of the other
binding operations.

Another important type of binding is the `computed()` binding, which makes it possible 
to create a binding that derives a value from other bindings. Computed bindings 
automatically monitor any bindings that were captured for changes, so they can be
`follow`ed or `when_change`d as with any other binding:

```Rust
    let binding         = bind(1);
    let binding_copy    = binding.clone();
    let one_more        = computed(move || binding_copy.get() + 1);

    let event_lifetime  = one_more.when_changed(notify(|| println!("Computed binding changed")));

    let value = one_more.get();     // == 2 (1 + 1)
    binding.set(2);                 // Prints 'Computed binding changed'
    binding.set(3);                 // Computed binding has not been read since the last notification so prints nothing
    let value = one_more.get();     // == 4 (3 + 1)
```

For collections of data, `flo_binding` uses the concept of a 'rope binding'. The 
general rope data type is provided by the [`flo_rope`](https://crates.io/crates/flo_rope)
crate. These bindings send differences rather than their full state when streaming and
are internally represented by a data structure that can perform deletions and insertions
efficiently. Unlike the traditional concept of a rope, they aren't limited to editing
strings, and can annotate their contents with attributes, which makes them suitable for
representing sequences of any kind, or sequences of rich text annotated with style
information.

```Rust
    let mutable_rope        = RopeBindingMut::<usize, ()>::new();
    let rope_copy           = RopeBinding::from_stream(mutable_rope.follow_changes());
    let mut rope_stream     = rope_copy.follow_changes();

    mutable_rope.replace(0..0, vec![1, 2, 3, 4]);

    let next            = rope_stream.next().await;                         // == RopeAction::Replace(0..0, vec![1,2,3,4]))

    let rope_len        = rope_copy.len();                                  // == 4
    let rope_content    = rope_copy.read_cells(0..4).collect::<Vec<_>>()    // == vec![1, 2, 3, 4]
```

The `flo_rope` library provides some extra functionality - for example, a way to create the
`RopeAction`s for a rope by using a diff algorithm on a changing sequence instead of just
reporting the changes as they arrive.

## Companion libraries

Aside from `flo_rope`, the [`desync`](https://crates.io/crates/desync) crate provides a 
novel approach to asynchronous code, including pipe operations that work very well with 
the streamed updates from the `follow()` function.

The [`flo_stream`](https://crates.io/crates/flo_stream) provides a pubsub system that
provides more flexible ways for distributing state through streams.

[`flo_scene`](https://github.com/logicalshift/flo_scene) is a runtime for building
complex systems out of entities that exchange messages with each other. It uses
`flo_binding` as an ergonomic way to exchange state information.
