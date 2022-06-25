use std::marker::{PhantomData};

///
/// A map binding is a type of computed binding created by the `BoundValueExt::map()` function
///
pub struct MapBinding<TValue, TMapValue, TMapFn> {
    _phantom: (PhantomData<TValue>, PhantomData<TMapValue>, PhantomData<TMapFn>)
}

impl<TValue, TMapValue, TMapFn> MapBinding<TValue, TMapValue, TMapFn>
where
    TValue: 'static + Clone + Send,
    TMapFn: 'static + Send + Sync + Fn(TValue) -> TMapValue 
{

}
