use wasi::clocks::monotonic_clock;
use wasi::io::poll::{self, Pollable};
use wasi::io::streams::{ InputStream, OutputStream, StreamError };
use wasi::random::random;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress,
    Ipv4SocketAddress, Ipv6SocketAddress, Network,
};
use wasi::sockets::tcp::TcpSocket;
use wasi::sockets::tcp_create_socket;
use wasi::sockets::udp::UdpSocket;
use wasi::sockets::udp_create_socket;
use std::ops::Range;

const TIMEOUT_NS: u64 = 1_000_000_000;

/// Execute the inner function with a randomly generated port.
/// To prevent random failures, we make a few attempts before giving up.
pub fn attempt_random_port<F>(
    local_address: IpAddress,
    mut f: F,
) -> Result<IpSocketAddress, ErrorCode>
where
    F: FnMut(IpSocketAddress) -> Result<(), ErrorCode>,
{
    const MAX_ATTEMPTS: u32 = 10;
    let mut i = 0;
    loop {
        i += 1;

        let port: u16 = generate_random_u16(1024..u16::MAX);
        let sock_addr = ip_socket_address_new(local_address, port);

        match f(sock_addr) {
            Ok(_) => return Ok(sock_addr),
            Err(e) if i >= MAX_ATTEMPTS => return Err(e),
            // Try again if the port is already taken. This can sometimes show up as `AccessDenied` on Windows.
            Err(ErrorCode::AddressInUse | ErrorCode::AccessDenied) => {}
            Err(e) => return Err(e),
        }
    }
}

pub fn tcp_socket_new(address_family: IpAddressFamily) -> Result<TcpSocket, ErrorCode> {
    tcp_create_socket::create_tcp_socket(address_family)
}

pub fn udp_socket_new(address_family: IpAddressFamily) -> Result<UdpSocket, ErrorCode> {
    udp_create_socket::create_udp_socket(address_family)
}

pub fn blocking_bind(
    sock: &TcpSocket,
    network: &Network,
    local_address: IpSocketAddress,
) -> Result<(), ErrorCode> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let sub = sock.subscribe();

    sock.start_bind(&network, local_address)?;

    loop {
        match sock.finish_bind() {
            Err(ErrorCode::WouldBlock) => block_until(&sub, &timeout)?,
            result => return result,
        }
    }
}

pub fn blocking_bind_udp(
    sock: &UdpSocket,
    network: &Network,
    local_address: IpSocketAddress,
) -> Result<(), ErrorCode> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let sub = sock.subscribe();

    sock.start_bind(&network, local_address)?;

    loop {
        match sock.finish_bind() {
            Err(ErrorCode::WouldBlock) => block_until(&sub, &timeout)?,
            result => return result,
        }
    }
}

pub fn blocking_bind_unspecified(sock: &UdpSocket, network: &Network) -> Result<(), ErrorCode> {
    let ip = new_unspecified(sock.address_family());
    let port = 0;

    blocking_bind_udp(sock, network, ip_socket_address_new(ip, port))
}

pub fn blocking_listen(sock: &TcpSocket) -> Result<(), ErrorCode> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let sub = sock.subscribe();

    sock.start_listen()?;

    loop {
        match sock.finish_listen() {
            Err(ErrorCode::WouldBlock) => block_until(&sub, &timeout)?,
            result => return result,
        }
    }
}

pub fn blocking_connect(
    sock: &TcpSocket,
    network: &Network,
    remote_address: IpSocketAddress,
) -> Result<(InputStream, OutputStream), ErrorCode> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let sub = sock.subscribe();

    sock.start_connect(&network, remote_address)?;

    loop {
        match sock.finish_connect() {
            Err(ErrorCode::WouldBlock) => block_until(&sub, &timeout)?,
            result => return result,
        }
    }
}

pub fn blocking_accept(sock: &TcpSocket) -> Result<(TcpSocket, InputStream, OutputStream), ErrorCode> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let sub = sock.subscribe();

    loop {
        match sock.accept() {
            Err(ErrorCode::WouldBlock) => block_until(&sub, &timeout)?,
            result => return result,
        }
    }
}

pub fn block_until(pollable: &Pollable, timeout: &Pollable) -> Result<(), ErrorCode> {
    let ready = poll::poll(&[pollable, timeout]);
    assert!(ready.len() > 0);
    match ready[0] {
        0 => Ok(()),
        1 => Err(ErrorCode::Timeout),
        _ => unreachable!(),
    }
}

pub const IPV4_BROADCAST: IpAddress = IpAddress::Ipv4((255, 255, 255, 255));

pub const IPV4_LOOPBACK: IpAddress = IpAddress::Ipv4((127, 0, 0, 1));
pub const IPV6_LOOPBACK: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 1));

pub const IPV4_UNSPECIFIED: IpAddress = IpAddress::Ipv4((0, 0, 0, 0));
pub const IPV6_UNSPECIFIED: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 0));

pub const IPV4_MAPPED_LOOPBACK: IpAddress =
    IpAddress::Ipv6((0, 0, 0, 0, 0, 0xFFFF, 0x7F00, 0x0001));

pub const fn ip_family(addr: &IpAddress) -> IpAddressFamily {
    match addr {
        IpAddress::Ipv4(_) => IpAddressFamily::Ipv4,
        IpAddress::Ipv6(_) => IpAddressFamily::Ipv6,
    }
}

pub const fn new_loopback(family: IpAddressFamily) -> IpAddress {
    match family {
        IpAddressFamily::Ipv4 => IPV4_LOOPBACK,
        IpAddressFamily::Ipv6 => IPV6_LOOPBACK,
    }
}

pub const fn new_unspecified(family: IpAddressFamily) -> IpAddress {
    match family {
        IpAddressFamily::Ipv4 => IPV4_UNSPECIFIED,
        IpAddressFamily::Ipv6 => IPV6_UNSPECIFIED,
    }
}

pub const fn ip_socket_address_new(ip: IpAddress, port: u16) -> IpSocketAddress {
    match ip {
        IpAddress::Ipv4(addr) => IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port,
            address: addr,
        }),
        IpAddress::Ipv6(addr) => IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port,
            address: addr,
            flow_info: 0,
            scope_id: 0,
        }),
    }
}

pub const fn ip_socket_address_ip(&sock_addr: &IpSocketAddress) -> IpAddress {
    match sock_addr {
        IpSocketAddress::Ipv4(addr) => IpAddress::Ipv4(addr.address),
        IpSocketAddress::Ipv6(addr) => IpAddress::Ipv6(addr.address),
    }
}

pub const fn port(sock_addr: &IpSocketAddress) -> u16 {
    match sock_addr {
        IpSocketAddress::Ipv4(addr) => addr.port,
        IpSocketAddress::Ipv6(addr) => addr.port,
    }
}

#[derive(Debug)]
pub struct IpAddressEq(pub IpAddress);

impl PartialEq for IpAddressEq {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IpAddressEq(IpAddress::Ipv4(left)), IpAddressEq(IpAddress::Ipv4(right))) => left == right,
            (IpAddressEq(IpAddress::Ipv6(left)), IpAddressEq(IpAddress::Ipv6(right))) => left == right,
            _ => false,
        }
    }
}

fn ipv4_socket_address_eq(this: &Ipv4SocketAddress, other: &Ipv4SocketAddress) -> bool {
    this.port == other.port && this.address == other.address
}

fn ipv6_socket_address_eq(this: &Ipv6SocketAddress, other: &Ipv6SocketAddress) -> bool {
    this.port == other.port
        && this.flow_info == other.flow_info
        && this.address == other.address
        && this.scope_id == other.scope_id
}

#[derive(Debug)]
pub struct IpSocketAddressEq(pub IpSocketAddress);

impl PartialEq for IpSocketAddressEq {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IpSocketAddressEq(IpSocketAddress::Ipv4(l0)), IpSocketAddressEq(IpSocketAddress::Ipv4(r0))) => ipv4_socket_address_eq(l0, r0),
            (IpSocketAddressEq(IpSocketAddress::Ipv6(l0)), IpSocketAddressEq(IpSocketAddress::Ipv6(r0))) => ipv6_socket_address_eq(l0, r0),
            _ => false,
        }
    }
}

pub fn generate_random_u16(range: Range<u16>) -> u16 {
    let start = range.start as u64;
    let end = range.end as u64;
    let port = start + (random::get_random_u64() % (end - start));
    port as u16
}

pub fn blocking_read_to_end(stream: &InputStream) -> Result<Vec<u8>, wasi::io::error::Error> {
    let mut data = vec![];
    loop {
        match stream.blocking_read(1024 * 1024) {
            Ok(chunk) => data.extend(chunk),
            Err(StreamError::Closed) => return Ok(data),
            Err(StreamError::LastOperationFailed(e)) => return Err(e),
        }
    }
}

pub fn blocking_write_util(stream: &OutputStream, mut bytes: &[u8]) -> Result<(), StreamError> {
    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let pollable = stream.subscribe();

    while !bytes.is_empty() {
        block_until(&pollable, &timeout).expect("write timed out");

        let permit = stream.check_write()?;

        let len = bytes.len().min(permit as usize);
        let (chunk, rest) = bytes.split_at(len);

        stream.write(chunk)?;

        stream.blocking_flush()?;

        bytes = rest;
    }
    Ok(())
}
