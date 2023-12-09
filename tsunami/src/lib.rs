pub mod cli;
pub mod net;
pub mod receiver;
pub mod worker;

pub type Port = u16;

pub enum Message {
    Payload(Vec<Port>),
    Break,
}

#[derive(Debug, Clone, Copy)]
pub struct PortInfo {
    status: PortStatus,
    retried: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortStatus {
    Open,
    Closed,
    Filtered,
    NotInspected,
}
