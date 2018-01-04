use super::traits::*;
use super::binding::*;

use std::sync::*;

///
/// A `BindRef` references another binding without needing to know precisely
/// what kind of binding it is. It is read-only, so mostly useful for passing
/// a binding around, particularly for computed bindings. Create one with
/// `BindRef::from(binding)`.
/// 
/// Cloning a `BindRef` will create another reference to the same binding.
/// 
pub struct BindRef<Target> {
    reference: Arc<Bound<Target>>
}

impl<Value> Bound<Value> for BindRef<Value> {
    #[inline]
    fn get(&self) -> Value {
        self.reference.get()
    }
}

impl<Value> Changeable for BindRef<Value> {
    #[inline]
    fn when_changed(&self, what: Arc<Notifiable>) -> Box<Releasable> {
        self.reference.when_changed(what)
    }
}

impl<Value> Clone for BindRef<Value> {
    fn clone(&self) -> Self {
        BindRef {
            reference: Arc::clone(&self.reference)
        }
    }
}

impl<Value> BindRef<Value> {
    ///
    /// Creates a new BindRef from a reference to an existing binding
    /// 
    #[inline]
    pub fn new<Binding: 'static+Clone+Bound<Value>>(binding: &Binding) -> BindRef<Value> {
        BindRef {
            reference: Arc::new(binding.clone())
        }
    }

    ///
    /// Creates a new BindRef from an existing binding
    /// 
    #[inline]
    pub fn from<Binding: 'static+Bound<Value>>(binding: Binding) -> BindRef<Value> {
        BindRef {
            reference: Arc::new(binding)
        }
    }

    ///
    /// Creates a new BindRef from an existing binding
    /// 
    #[inline]
    pub fn from_arc<Binding: 'static+Bound<Value>>(binding_ref: Arc<Binding>) -> BindRef<Value> {
        BindRef {
            reference: binding_ref
        }
    }
}

impl<'a, Value> From<&'a BindRef<Value>> for BindRef<Value> {
    #[inline]
    fn from(val: &'a BindRef<Value>) -> Self {
        BindRef::clone(val)
    }
}

impl<Value: 'static+Clone+Send+PartialEq> From<Binding<Value>> for BindRef<Value> {
    fn from(val: Binding<Value>) -> Self {
        BindRef {
            reference: Arc::new(val)
        }
    }
}

impl<'a, Value: 'static+Clone+PartialEq+Send> From<&'a Binding<Value>> for BindRef<Value> {
    fn from(val: &'a Binding<Value>) -> Self {
        BindRef {
            reference: Arc::new(val.clone())
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::*;

    #[test]
    fn bindref_matches_core_value() {
        let mut bind    = bind(1);
        let bind_ref    = BindRef::from(bind.clone());

        assert!(bind_ref.get() == 1);

        bind.set(2);

        assert!(bind_ref.get() == 2);
    }

    #[test]
    fn bindref_matches_core_value_when_created_from_ref() {
        let mut bind    = bind(1);
        let bind_ref    = BindRef::new(&bind);

        assert!(bind_ref.get() == 1);

        bind.set(2);

        assert!(bind_ref.get() == 2);
    }
}
