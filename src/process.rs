pub(crate) struct Process {
    pub(crate) id: u32,
    pub(crate) ip: String,
    pub(crate) port: u32,
    pub(crate) leader: bool,
    pub(crate) me: bool,
}