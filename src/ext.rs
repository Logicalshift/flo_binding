use crate::traits::*;
use crate::bindref::*;
use crate::computed::*;
use crate::map_binding::*;

impl<TBinding> BoundValueMapExt for TBinding
where
    TBinding:   'static + Clone + Bound,
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

impl<TBinding> BoundValueComputeExt for TBinding
where
    TBinding:   'static + Clone + Bound,
{
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue
    {
        let input   = self.clone();
        let binding = ComputedBinding::new(move || map_fn(&input));

        BindRef::new(&binding)
    }
}

impl<TBindingA, TBindingB> BoundValueComputeExt for (TBindingA, TBindingB)
where
    TBindingA:  'static + Clone + Bound,
    TBindingB:  'static + Clone + Bound,
{
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue
    {
        let input   = self.clone();
        let binding = ComputedBinding::new(move || map_fn(&input));

        BindRef::new(&binding)
    }
}

impl<TBindingA, TBindingB, TBindingC> BoundValueComputeExt for (TBindingA, TBindingB, TBindingC)
where
    TBindingA:  'static + Clone + Bound,
    TBindingB:  'static + Clone + Bound,
    TBindingC:  'static + Clone + Bound,
{
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue
    {
        let input   = self.clone();
        let binding = ComputedBinding::new(move || map_fn(&input));

        BindRef::new(&binding)
    }
}

impl<TBindingA, TBindingB, TBindingC, TBindingD> BoundValueComputeExt for (TBindingA, TBindingB, TBindingC, TBindingD)
where
    TBindingA:  'static + Clone + Bound,
    TBindingB:  'static + Clone + Bound,
    TBindingC:  'static + Clone + Bound,
    TBindingD:  'static + Clone + Bound,
{
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue
    {
        let input   = self.clone();
        let binding = ComputedBinding::new(move || map_fn(&input));

        BindRef::new(&binding)
    }
}

impl<TBindingA, TBindingB, TBindingC, TBindingD, TBindingE> BoundValueComputeExt for (TBindingA, TBindingB, TBindingC, TBindingD, TBindingE)
where
    TBindingA:  'static + Clone + Bound,
    TBindingB:  'static + Clone + Bound,
    TBindingC:  'static + Clone + Bound,
    TBindingD:  'static + Clone + Bound,
    TBindingE:  'static + Clone + Bound,
{
    fn compute<TResultValue, TComputeFn>(&self, map_fn: TComputeFn) -> BindRef<TResultValue>
    where
        TResultValue:   'static + Clone + Send,
        TComputeFn:     'static + Send + Sync + Fn(&Self) -> TResultValue
    {
        let input   = self.clone();
        let binding = ComputedBinding::new(move || map_fn(&input));

        BindRef::new(&binding)
    }
}
