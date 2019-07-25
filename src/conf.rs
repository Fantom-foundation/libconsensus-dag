// Config module

pub struct DAGconfig {
    pub(crate) inner_port: u16,
    pub(crate) service_port: u16,
    pub(crate) callback_timeout: u64,
}

impl Default for DAGconfig {
    fn default() -> Self {
        return DAGconfig {
            inner_port: 9000,
            service_port: 12000,
            callback_timeout: 100,
        };
    }
}
