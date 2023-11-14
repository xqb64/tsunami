use anyhow::{bail, Result};
use pnet::packet::{
    ip::IpNextHeaderProtocols,
    ipv4::{Ipv4Flags, MutableIpv4Packet},
    tcp::{MutableTcpPacket, TcpFlags},
};
use raw_socket::{tokio::prelude::RawSocket, Domain, Protocol, Type};
use std::net::Ipv4Addr;

pub const IP_HDR_LEN: u16 = 20;
pub const TCP_HDR_LEN: u16 = 20;
pub const IPPROTO_RAW: i32 = 255;

pub fn build_ipv4_packet(buf: &mut [u8], dest: Ipv4Addr, id: u16) -> MutableIpv4Packet {
    use pnet::packet::ipv4::checksum;

    let mut packet = MutableIpv4Packet::new(buf).unwrap();

    packet.set_version(4);
    packet.set_ttl(u8::MAX);
    packet.set_header_length(5); /* n * 32 bits. */

    /* We are setting the identification field to the TTL
     * that we later use to map responses back to correct hops. */
    packet.set_identification(id);
    packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    packet.set_destination(dest);
    packet.set_flags(Ipv4Flags::DontFragment);
    packet.set_total_length(IP_HDR_LEN + TCP_HDR_LEN);
    packet.set_checksum(checksum(&packet.to_immutable()));

    packet
}

pub fn build_tcp_packet(buf: &mut [u8], destination: Ipv4Addr, port: u16) -> MutableTcpPacket {
    use pnet::packet::tcp::ipv4_checksum;

    let mut packet = MutableTcpPacket::new(buf).unwrap();
    packet.set_source(0x1337_u16);
    packet.set_destination(port);
    packet.set_sequence(rand::random::<u32>());
    packet.set_data_offset(5);
    packet.set_flags(TcpFlags::SYN);
    packet.set_window(0x7110_u16);
    packet.set_checksum(ipv4_checksum(
        &packet.to_immutable(),
        &Ipv4Addr::new(192, 168, 1, 64),
        &destination,
    ));

    packet
}

pub fn create_recv_sock() -> Result<RawSocket> {
    match RawSocket::new(Domain::ipv4(), Type::raw(), Protocol::tcp().into()) {
        Ok(sock) => Ok(sock),
        Err(_) => bail!("couldn't create the socket"),
    }
}

pub fn create_send_sock() -> Result<RawSocket> {
    match RawSocket::new(
        Domain::ipv4(),
        Type::raw(),
        Protocol::from(IPPROTO_RAW).into(),
    ) {
        Ok(sock) => Ok(sock),
        Err(_) => bail!("couldn't create the socket"),
    }
}
