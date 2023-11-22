use std::collections::HashSet;

pub mod cli;
pub mod net;
pub mod receiver;
pub mod worker;

pub enum Message {
    Payload(HashSet<u16>),
    Break,
}
