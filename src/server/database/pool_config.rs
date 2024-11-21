pub(crate) struct PoolConfig {
    /// connection pool size
    size: usize,
    /// connection string
    conn_str: &'static str,
}
