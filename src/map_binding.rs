use crate::traits::*;
use crate::computed::*;

use std::marker::{PhantomData};

use std::sync::*;

///
/// A map binding is a type of computed binding created by the `BoundValueExt::map()` function
///
#[derive(Clone)]
pub struct MapBinding<TBinding: ?Sized, TMapValue, TMapFn> {
    /// Reference to a computed binding that performs the map function
    computed: Arc<dyn Bound<Value=TMapValue>>,

    /// Phantom data (we present an interface on the outside that is like we re-implemented a computed binding, so future versions of flo_binding can do that without changing the API)
    _phantom: (PhantomData<TBinding>, PhantomData<TMapFn>),
}

impl<TBinding, TMapValue, TMapFn> MapBinding<TBinding, TMapValue, TMapFn>
where
    TBinding:   'static + Bound,
    TMapFn:     'static + Send + Sync + Fn(TBinding::Value) -> TMapValue,
    TMapValue:  'static + Send + Clone,
{
    ///
    /// Creates a new map binding
    ///
    pub (crate) fn new(binding: TBinding, map_fn: TMapFn) -> MapBinding<TBinding, TMapValue, TMapFn> {
        let computed_map = ComputedBinding::new(move || map_fn(binding.get()));

        MapBinding {
            computed: Arc::new(computed_map),
            _phantom: (PhantomData, PhantomData)
        }
    }
}

impl<TBinding, TMapValue, TMapFn> Changeable for MapBinding<TBinding, TMapValue, TMapFn>
where
    TBinding:   'static + Bound,
    TMapFn:     'static + Send + Sync + Fn(TBinding::Value) -> TMapValue,
    TMapValue:  'static + Send + Clone,
{
    #[inline]
    fn when_changed(&self, what: Arc<dyn Notifiable>) -> Box<dyn Releasable> {
        self.computed.when_changed(what)
    }
}

impl<TBinding, TMapValue, TMapFn> Bound for MapBinding<TBinding, TMapValue, TMapFn>
where
    TBinding:   'static + Bound,
    TMapFn:     'static + Send + Sync + Fn(TBinding::Value) -> TMapValue,
    TMapValue:  'static + Send + Clone,
{
    type Value = TMapValue;

    #[inline]
    fn get(&self) -> Self::Value {
        self.computed.get()
    }

    #[inline]
    fn watch(&self, what: Arc<dyn Notifiable>) -> Arc<dyn Watcher<Self::Value>> {
        self.computed.watch(what)
    }
}
