pub mod cli;
pub mod net;
pub mod receiver;
pub mod worker;

pub enum Message {
    Payload(Vec<u16>),
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

pub type Port = u16;
