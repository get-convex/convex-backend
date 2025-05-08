//! An AsyncLru that can store multiple types of data at the same time, while
//! still offering type safety.

use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    hash::{
        DefaultHasher,
        Hash,
        Hasher,
    },
    sync::Arc,
};

use anyhow::Context as _;
use common::runtime::Runtime;

use crate::async_lru::{
    AsyncLru,
    SingleValueGenerator,
    SizedValue,
};

pub trait LruKey: Any + Hash + Eq + Clone + Send + Sync + Debug {
    type Value: SizedValue + Send + Sync;
}

trait BaseLruKey: Any + Send + Sync + Debug {
    fn eq(&self, other: &dyn BaseLruKey) -> bool;
    fn key_hash(&self) -> u64;
    fn box_clone(&self) -> GenericKey;
}

impl<T: LruKey> BaseLruKey for T {
    fn eq(&self, other: &dyn BaseLruKey) -> bool {
        <dyn Any>::downcast_ref::<Self>(other).is_some_and(|other| self == other)
    }

    fn key_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.type_id().hash(&mut hasher);
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn box_clone(&self) -> GenericKey {
        Box::new(self.clone())
    }
}

type GenericKey = Box<dyn BaseLruKey>;

impl Hash for GenericKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key_hash().hash(state)
    }
}

impl PartialEq for GenericKey {
    fn eq(&self, other: &Self) -> bool {
        BaseLruKey::eq(&**self, &**other)
    }
}

impl Eq for GenericKey {}

impl Clone for GenericKey {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

trait BaseLruValue: Any + SizedValue + Send + Sync {}
impl<T: Any + SizedValue + Send + Sync> BaseLruValue for T {}

#[derive(Clone)]
pub struct MultiTypeAsyncLru<RT: Runtime> {
    inner: AsyncLru<RT, GenericKey, dyn BaseLruValue>,
}

impl<RT: Runtime> MultiTypeAsyncLru<RT> {
    pub fn new(rt: RT, max_size: u64, concurrency: usize, label: &'static str) -> Self {
        Self {
            inner: AsyncLru::new(rt, max_size, concurrency, label),
        }
    }

    pub fn size(&self) -> u64 {
        self.inner.size()
    }

    pub async fn get<Key: LruKey + Clone, V: 'static>(
        &self,
        key: Key,
        value_generator: SingleValueGenerator<V>,
    ) -> anyhow::Result<Arc<Key::Value>>
    where
        Arc<Key::Value>: From<V>,
    {
        let key_ = key.clone();
        let result = self
            .inner
            .get_and_prepopulate(
                Box::new(key_),
                Box::pin(async move {
                    let mut hashmap = HashMap::new();
                    hashmap.insert(
                        Box::new(key) as GenericKey,
                        value_generator
                            .await
                            .map(|v| <Arc<Key::Value>>::from(v) as Arc<dyn BaseLruValue>),
                    );
                    hashmap
                }),
            )
            .await?;
        (result as Arc<dyn Any + Send + Sync>)
            .downcast()
            .ok()
            .context("MultiTypeAsyncLru error: cached value was wrong type")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::FutureExt;
    use runtime::testing::TestRuntime;

    use super::{
        LruKey,
        MultiTypeAsyncLru,
    };
    use crate::async_lru::SizedValue;

    #[derive(Hash, PartialEq, Eq, Clone, Debug)]
    struct Key1(u32);
    #[derive(Hash, PartialEq, Eq, Clone, Debug)]
    struct Key2(u32);
    #[derive(PartialEq, Debug)]
    struct Value1(String);
    #[derive(PartialEq, Debug)]
    struct Value2(String);
    impl SizedValue for Value1 {
        fn size(&self) -> u64 {
            0
        }
    }
    impl SizedValue for Value2 {
        fn size(&self) -> u64 {
            0
        }
    }
    impl LruKey for Key1 {
        type Value = Value1;
    }
    impl LruKey for Key2 {
        type Value = Value2;
    }

    #[convex_macro::test_runtime]
    async fn test_multi_type_lru(rt: TestRuntime) -> anyhow::Result<()> {
        let lru = MultiTypeAsyncLru::new(rt, 100, 1, "lru");
        let v = lru
            .get(Key1(123), async move { Ok(Value1("foo".into())) }.boxed())
            .await?;
        assert_eq!(*v, Value1("foo".into()));
        // Fetching the same key again should return the same value (and not run the
        // computation)
        let v2 = lru
            .get::<Key1, Value1>(Key1(123), async move { panic!() }.boxed())
            .await?;
        assert!(Arc::ptr_eq(&v, &v2));

        // A different key with the same type should recompute
        let v = lru
            .get(Key1(456), async move { Ok(Value1("bar".into())) }.boxed())
            .await?;
        assert_eq!(*v, Value1("bar".into()));

        // Different key types never overlap
        let v = lru
            .get(
                Key2(123),
                async move { Ok(Value2("foofoo".into())) }.boxed(),
            )
            .await?;
        assert_eq!(*v, Value2("foofoo".into()));
        let v2 = lru
            .get::<Key2, Value2>(Key2(123), async move { panic!() }.boxed())
            .await?;
        assert!(Arc::ptr_eq(&v, &v2));
        Ok(())
    }
}
