use crate::traits::*;
use crate::map_binding::*;

impl<TBinding> BoundValueExt for TBinding
where
    TBinding:   'static + Clone + Bound
{
    type Value = TBinding::Value;

    fn map_binding<TMapValue, TMapFn>(&self, map_fn: TMapFn) -> MapBinding<Self, TMapValue, TMapFn>
    where
        TMapValue:  'static + Clone + Send,
        TMapFn:     'static + Send + Sync + Fn(Self::Value) -> TMapValue
    {
        MapBinding::new(self.clone(), map_fn)
    }
}
