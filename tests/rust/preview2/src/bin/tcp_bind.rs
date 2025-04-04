use wasi::clocks::monotonic_clock;
use wasi::io::poll::{self, Pollable};
use wasi::io::streams::{ InputStream, OutputStream };
use wasi::random::random;
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress,
    Ipv4SocketAddress, Ipv6SocketAddress, Network,
};
use wasi::sockets::tcp::TcpSocket;
use wasi::sockets::tcp_create_socket;
use std::ops::Range;

const TIMEOUT_NS: u64 = 1_000_000_000;

/// Bind a socket and let the system determine a port.
fn test_tcp_bind_ephemeral_port(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock = tcp_socket_new(ip_family(&ip)).unwrap();
    blocking_bind(&sock, net, bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(IpAddressEq(ip_socket_address_ip(&bind_addr)),
        IpAddressEq(ip_socket_address_ip(&bound_addr)));
    assert_ne!(port(&bind_addr), port(&bound_addr));
}

/// Bind a socket on a specified port.
fn test_tcp_bind_specific_port(net: &Network, ip: IpAddress) {
    let sock = tcp_socket_new(ip_family(&ip)).unwrap();

    let bind_addr =
        attempt_random_port(ip, |bind_addr| blocking_bind(&sock, net, bind_addr)).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(IpAddressEq(ip_socket_address_ip(&bind_addr)),
        IpAddressEq(ip_socket_address_ip(&bound_addr)));
    assert_eq!(port(&bind_addr), port(&bound_addr));
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_tcp_bind_addrinuse(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock1 = tcp_socket_new(ip_family(&ip)).unwrap();
    blocking_bind(&sock1, net, bind_addr).unwrap();
    blocking_listen(&sock1).unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = tcp_socket_new(ip_family(&ip)).unwrap();
    assert_eq!(
        blocking_bind(&sock2, net, bound_addr),
        Err(ErrorCode::AddressInUse)
    );
}

// The WASI runtime should set SO_REUSEADDR for us
fn test_tcp_bind_reuseaddr(net: &Network, ip: IpAddress) {
    let client = tcp_socket_new(ip_family(&ip)).unwrap();

    let bind_addr = {
        let listener1 = tcp_socket_new(ip_family(&ip)).unwrap();

        let bind_addr =
            attempt_random_port(ip, |bind_addr| blocking_bind(&listener1, net, bind_addr)).unwrap();

        blocking_listen(&listener1).unwrap();

        let connect_addr =
            ip_socket_address_new(new_loopback(ip_family(&ip)), port(&bind_addr));
        blocking_connect(&client, net, connect_addr).unwrap();

        let (accepted_connection, accepted_input, accepted_output) =
            blocking_accept(&listener1).unwrap();
        accepted_output.blocking_write_zeroes_and_flush(10).unwrap();
        drop(accepted_input);
        drop(accepted_output);
        drop(accepted_connection);
        drop(listener1);

        bind_addr
    };

    {
        let listener2 = tcp_socket_new(ip_family(&ip)).unwrap();

        // If SO_REUSEADDR was configured correctly, the following lines shouldn't be
        // affected by the TIME_WAIT state of the just closed `listener1` socket:
        blocking_bind(&listener2, net, bind_addr).unwrap();
        blocking_listen(&listener2).unwrap();
    }

    drop(client);
}

// Try binding to an address that is not configured on the system.
fn test_tcp_bind_addrnotavail(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock = tcp_socket_new(ip_family(&ip)).unwrap();

    assert_eq!(
        blocking_bind(&sock, net, bind_addr),
        Err(ErrorCode::AddressNotBindable)
    );
}

/// Bind should validate the address family.
fn test_tcp_bind_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IPV4_LOOPBACK,
    };

    let sock = tcp_socket_new(family).unwrap();
    let result = blocking_bind(&sock, net, ip_socket_address_new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

/// Bind only works on unicast addresses.
fn test_tcp_bind_non_unicast(net: &Network) {
    let ipv4_broadcast = ip_socket_address_new(IPV4_BROADCAST, 0);
    let ipv4_multicast = ip_socket_address_new(IpAddress::Ipv4((224, 254, 0, 0)), 0);
    let ipv6_multicast = ip_socket_address_new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), 0);

    let sock_v4 = tcp_socket_new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = tcp_socket_new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(
        blocking_bind(&sock_v4, net, ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        blocking_bind(&sock_v4, net, ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        blocking_bind(&sock_v6, net, ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_bind_dual_stack(net: &Network) {
    let sock = tcp_socket_new(IpAddressFamily::Ipv6).unwrap();
    let addr = ip_socket_address_new(IPV4_MAPPED_LOOPBACK, 0);

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(
        blocking_bind(&sock, net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn main() {
    const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
    const RESERVED_IPV6_ADDRESS: IpAddress = IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

    let net = instance_network();

    test_tcp_bind_ephemeral_port(&&net, IPV4_LOOPBACK);
    test_tcp_bind_ephemeral_port(&&net, IPV6_LOOPBACK);
    test_tcp_bind_ephemeral_port(&&net, IPV4_UNSPECIFIED);
    test_tcp_bind_ephemeral_port(&&net, IPV6_UNSPECIFIED);

    test_tcp_bind_specific_port(&&net, IPV4_LOOPBACK);
    test_tcp_bind_specific_port(&&net, IPV6_LOOPBACK);
    test_tcp_bind_specific_port(&&net, IPV4_UNSPECIFIED);
    test_tcp_bind_specific_port(&&net, IPV6_UNSPECIFIED);

    test_tcp_bind_reuseaddr(&net, IPV4_LOOPBACK);
    test_tcp_bind_reuseaddr(&net, IPV6_LOOPBACK);

    test_tcp_bind_addrinuse(&net, IPV4_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IPV6_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IPV4_UNSPECIFIED);
    test_tcp_bind_addrinuse(&net, IPV6_UNSPECIFIED);

    test_tcp_bind_addrnotavail(&net, RESERVED_IPV4_ADDRESS);
    test_tcp_bind_addrnotavail(&net, RESERVED_IPV6_ADDRESS);

    test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv4);
    test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv6);

    test_tcp_bind_non_unicast(&net);

    test_tcp_bind_dual_stack(&net);
}

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
struct IpAddressEq(IpAddress);

impl PartialEq for IpAddressEq {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IpAddressEq(IpAddress::Ipv4(left)), IpAddressEq(IpAddress::Ipv4(right))) => left == right,
            (IpAddressEq(IpAddress::Ipv6(left)), IpAddressEq(IpAddress::Ipv6(right))) => left == right,
            _ => false,
        }
    }
}

fn generate_random_u16(range: Range<u16>) -> u16 {
    let start = range.start as u64;
    let end = range.end as u64;
    let port = start + (random::get_random_u64() % (end - start));
    port as u16
}
