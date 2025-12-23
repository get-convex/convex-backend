//! Provides an equivalent to the [`v8::Isolate::set_slot`] API that is
//! context-local. [`v8::Context::set_slot`] doesn't work because it only offers
//! access behind an Rc, which is inconvenient.

use std::{
    collections::HashMap,
    hash::{
        Hash,
        Hasher,
    },
    rc::Rc,
};

use deno_core::v8;

// size > 0 so that pointers are unique
struct ContextIdSlot(#[allow(dead_code)] u8);
struct ContextId(Rc<ContextIdSlot>);
impl PartialEq for ContextId {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ContextId {}
impl Hash for ContextId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}
impl ContextId {
    fn of(context: &v8::Context) -> Self {
        if let Some(slot) = context.get_slot::<ContextIdSlot>() {
            return Self(slot);
        }
        let slot = Rc::new(ContextIdSlot(0));
        assert!(context.set_slot(slot.clone()).is_none());
        Self(slot)
    }
}

struct PerContext<T> {
    by_context: HashMap<ContextId, T>,
}
impl<T> PerContext<T> {
    fn new() -> Self {
        Self {
            by_context: HashMap::new(),
        }
    }

    fn get(&self, context: &v8::Context) -> Option<&T> {
        self.by_context.get(&ContextId::of(context))
    }

    fn get_mut(&mut self, context: &v8::Context) -> Option<&mut T> {
        self.by_context.get_mut(&ContextId::of(context))
    }

    fn set(&mut self, context: &v8::Context, value: T) -> bool {
        self.by_context
            .insert(ContextId::of(context), value)
            .is_none()
    }

    fn remove(&mut self, context: &v8::Context) -> Option<T> {
        self.by_context.remove(&ContextId::of(context))
    }
}

pub(crate) trait GetContextSlot {
    fn get_context_slot<'a, T: 'static>(&self, isolate: &'a v8::Isolate) -> Option<&'a T>;
    fn get_context_slot_mut<'a, T: 'static>(
        &self,
        isolate: &'a mut v8::Isolate,
    ) -> Option<&'a mut T>;
    fn set_context_slot<T: 'static>(&self, isolate: &mut v8::Isolate, value: T) -> bool;
    fn remove_context_slot<T: 'static>(&self, isolate: &mut v8::Isolate) -> Option<T>;
}

impl GetContextSlot for v8::Context {
    fn get_context_slot<'a, T: 'static>(&self, isolate: &'a v8::Isolate) -> Option<&'a T> {
        isolate
            .get_slot::<PerContext<T>>()
            .and_then(|slot| slot.get(self))
    }

    fn get_context_slot_mut<'a, T: 'static>(
        &self,
        isolate: &'a mut v8::Isolate,
    ) -> Option<&'a mut T> {
        isolate
            .get_slot_mut::<PerContext<T>>()
            .and_then(|slot| slot.get_mut(self))
    }

    fn set_context_slot<T: 'static>(&self, isolate: &mut v8::Isolate, value: T) -> bool {
        if let Some(slot) = isolate.get_slot_mut::<PerContext<T>>() {
            return slot.set(self, value);
        }
        let mut slot = PerContext::<T>::new();
        slot.set(self, value);
        assert!(isolate.set_slot(slot));
        true
    }

    fn remove_context_slot<T: 'static>(&self, isolate: &mut v8::Isolate) -> Option<T> {
        isolate
            .get_slot_mut::<PerContext<T>>()
            .and_then(|slot| slot.remove(self))
    }
}
