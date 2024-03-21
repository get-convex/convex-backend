use std::{
    ops::Deref,
    sync::Arc,
};

use parking_lot::Mutex;

/// The client pool is a generic that allows us to workaround limitations of
/// individual clients implementations, such as AWS Lambda rust client. It
/// limits the concurrency to each individual client and transparently creates
/// new clients and multiplexes requests. It is currently implemented using a
/// mutex walking over all clients on every get(). Thus it is not recommended if
/// you need to do hundreds of thousands of requests per second or shed load
/// over thousands of clients.
pub struct ClientPool<C> {
    create_client: Arc<dyn Fn() -> C + Send + Sync>,
    max_client_concurrency: usize,

    clients: Arc<Mutex<Vec<Arc<C>>>>,
}

impl<C> ClientPool<C> {
    pub fn new(
        create_client: impl Fn() -> C + Send + Sync + 'static,
        max_client_concurrency: usize,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            max_client_concurrency > 0,
            "max_client_concurrency must be positive"
        );
        Ok(Self {
            create_client: Arc::new(create_client),
            max_client_concurrency,
            clients: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn get(&self) -> BorrowedClient<C> {
        let mut clients = self.clients.lock();
        // Check if any of the existing clients have capacity.
        for client in clients.iter() {
            // The pool holds one reference. The remaining references are from
            // borrowed clients.
            if Arc::strong_count(client) - 1 < self.max_client_concurrency {
                return BorrowedClient {
                    inner: client.clone(),
                };
            }
        }

        // Create a new client.
        let client = Arc::new((self.create_client)());
        clients.push(client.clone());
        BorrowedClient { inner: client }
    }
}

pub struct BorrowedClient<C> {
    inner: Arc<C>,
}

impl<C> Deref for BorrowedClient<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{
            AtomicUsize,
            Ordering,
        },
        Arc,
    };

    use crate::client_pool::ClientPool;
    #[test]
    fn test_client_pool() {
        struct TestClient {
            id: usize,
        }
        let client_count = Arc::new(AtomicUsize::new(0));
        let _client_count = client_count.clone();
        let create_client = move || {
            let id = _client_count.fetch_add(1, Ordering::SeqCst);
            TestClient { id }
        };
        // Create a pool with concurrency of 5.
        let pool = ClientPool::new(create_client, 5).unwrap();

        // Borrow 5 clients. We should only create one client.
        let mut clients: Vec<_> = (0..5).map(|_| pool.get()).collect();
        assert!(client_count.load(Ordering::SeqCst) == 1);
        assert!(clients.iter().all(|c| c.id == 0));

        // Drop a borrowed client and borrow another one. We should still only
        // need a single underling client.
        clients.pop();
        let new_client = pool.get();
        assert!(client_count.load(Ordering::SeqCst) == 1);
        assert!(new_client.id == 0);
        clients.push(new_client);

        // Borrowing another client should now trigger us creation of another
        // underlying client.
        let new_client = pool.get();
        assert!(client_count.load(Ordering::SeqCst) == 2);
        assert!(new_client.id == 1);
        clients.push(new_client);

        // Borrowing 9 more should result in 3 underlying clients.
        clients.extend((0..9).map(|_| pool.get()));
        assert!(client_count.load(Ordering::SeqCst) == 3);
        assert!(clients.iter().all(|c| c.id < 3));

        // Drop all clients, allocated another 10. Should only use the first two
        // underlying clients.
        clients.clear();
        clients.extend((0..10).map(|_| pool.get()));
        assert!(client_count.load(Ordering::SeqCst) == 3);
        assert!(clients.iter().all(|c| c.id < 2));
    }
}
