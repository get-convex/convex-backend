#[derive(Clone, Copy)]
pub struct ElleConfig {
    pub seed: u64,
    pub num_clients: usize,

    pub num_tx: usize,
    pub max_concurrent_tx: usize,

    pub expected_disconnect_duration: u32,

    pub client_read_weight: u32,
    pub client_write_weight: u32,
    pub server_write_weight: u32,
    pub disconnect_client_weight: u32,
}

impl Default for ElleConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            num_clients: 4,
            num_tx: 32,
            max_concurrent_tx: 4,

            expected_disconnect_duration: 4,

            client_read_weight: 10,
            client_write_weight: 5,
            server_write_weight: 1,
            disconnect_client_weight: 8,
        }
    }
}
