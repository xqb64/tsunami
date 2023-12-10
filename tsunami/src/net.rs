use crate::{error_and_bail, Port};
use anyhow::{bail, Result};
use libc::{
    addrinfo, freeaddrinfo, gai_strerror, getaddrinfo, getnameinfo, sockaddr, socklen_t, AF_INET,
    NI_MAXHOST, NI_NUMERICHOST,
};
use pnet::packet::{
    ip::IpNextHeaderProtocols,
    ipv4::{Ipv4Flags, MutableIpv4Packet},
    tcp::{MutableTcpPacket, TcpFlags},
};
use raw_socket::{tokio::prelude::RawSocket, Domain, Protocol, Type};
use std::{
    ffi::{CStr, CString},
    net::{IpAddr, Ipv4Addr},
};

pub const IP_HDR_LEN: u16 = 20;
pub const TCP_HDR_LEN: u16 = 20;
pub const IPPROTO_RAW: i32 = 255;

pub async fn dns_lookup(hostname: &str) -> Result<IpAddr> {
    /* prepare the hints for the getaddrinfo call */
    let hints = addrinfo {
        ai_family: AF_INET,
        ai_socktype: 0,
        ai_protocol: 0,
        ai_flags: 0,
        ai_addrlen: 0,
        ai_canonname: std::ptr::null_mut(),
        ai_addr: std::ptr::null_mut(),
        ai_next: std::ptr::null_mut(),
    };
    let mut res: *mut addrinfo = std::ptr::null_mut();
    let c_hostname = CString::new(hostname)?;

    /* perform the DNS lookup */
    let err = unsafe { getaddrinfo(c_hostname.as_ptr(), std::ptr::null(), &hints, &mut res) };
    if err != 0 {
        /* if the lookup failed, return the error */
        let err_str = unsafe { CStr::from_ptr(gai_strerror(err)).to_str()? };
        error_and_bail!("DNS lookup for host {hostname} failed: {err_str}");
    }

    /* res now points to a linked list of addrinfo structures */
    /* convert the IP address from the first addrinfo structure to a string */
    let addr = unsafe { (*res).ai_addr as *const sockaddr };
    let mut host = [0; NI_MAXHOST as usize];

    /* use getnameinfo to convert the address into a string */
    let s = unsafe {
        getnameinfo(
            addr,
            (*res).ai_addrlen,
            host.as_mut_ptr(),
            host.len() as socklen_t,
            /* not interested in service info */
            std::ptr::null_mut(),
            0,
            /* return the numeric form of the hostname */
            NI_NUMERICHOST,
        )
    };

    /* free the mem allocated by getaddrinfo */
    unsafe { freeaddrinfo(res) };

    if s != 0 {
        /* if the conversion failed,error_and_bail */
        let err_str = unsafe { CStr::from_ptr(gai_strerror(s)).to_str()? };
        error_and_bail!("address conversion for host {hostname} failed: {err_str}");
    }

    /* convert the C string to a Rust IpAddr and return it */
    let c_str = unsafe { CStr::from_ptr(host.as_ptr()) };
    Ok(c_str.to_str()?.to_string().parse::<IpAddr>()?)
}

pub fn build_ipv4_packet(buf: &mut [u8], dest: Ipv4Addr) -> MutableIpv4Packet {
    use pnet::packet::ipv4::checksum;

    let mut packet = MutableIpv4Packet::new(buf).unwrap();

    packet.set_version(4);
    packet.set_ttl(u8::MAX);
    packet.set_header_length(5); /* n * 32 bits. */
    packet.set_identification(rand::random::<u16>());
    packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    packet.set_destination(dest);
    packet.set_flags(Ipv4Flags::DontFragment);
    packet.set_total_length(IP_HDR_LEN + TCP_HDR_LEN);
    packet.set_checksum(checksum(&packet.to_immutable()));

    packet
}

pub fn build_tcp_packet(
    buf: &mut [u8],
    destination: Ipv4Addr,
    port: Port,
    src_ip_addr: Ipv4Addr,
) -> Result<MutableTcpPacket> {
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
        &src_ip_addr,
        &destination,
    ));

    Ok(packet)
}

pub fn create_recv_sock() -> Result<RawSocket> {
    match RawSocket::new(Domain::ipv4(), Type::raw(), Protocol::tcp().into()) {
        Ok(sock) => Ok(sock),
        Err(_) => error_and_bail!("couldn't create the socket"),
    }
}

pub fn create_send_sock() -> Result<RawSocket> {
    match RawSocket::new(
        Domain::ipv4(),
        Type::raw(),
        Protocol::from(IPPROTO_RAW).into(),
    ) {
        Ok(sock) => Ok(sock),
        Err(_) => error_and_bail!("couldn't create the socket"),
    }
}

pub async fn to_ipaddr(target: &str) -> Result<Ipv4Addr> {
    match target.parse::<Ipv4Addr>() {
        Ok(addr) => Ok(addr),
        Err(_) => match dns_lookup(target).await {
            Ok(ip_addr) => match ip_addr {
                IpAddr::V4(addr) => Ok(addr),
                IpAddr::V6(_) => bail!("not implemented for ipv6."),
            },
            Err(_) => error_and_bail!("couldn't resolve the hostname {target}"),
        },
    }
}

pub fn get_default_gateway_interface() -> Result<IpAddr> {
    use pnet::datalink;
    use std::process::Command;

    let output = String::from_utf8(
        Command::new("ip")
            .args(["route", "show", "default"])
            .output()?
            .stdout,
    )?;

    let default_gateway = output
        .split_whitespace()
        .nth(2)
        .unwrap()
        .parse::<IpAddr>()?;

    let mut interface_ip_addr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    for interface in datalink::interfaces() {
        for ip_net in interface.ips {
            if ip_net.contains(default_gateway) {
                interface_ip_addr = ip_net.ip();
            }
        }
    }

    Ok(interface_ip_addr)
}
