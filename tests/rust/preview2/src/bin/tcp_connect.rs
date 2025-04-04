use wasi_tests::tcp_helpers::{ IPV4_BROADCAST, IPV4_LOOPBACK, IPV4_MAPPED_LOOPBACK,
    IPV6_LOOPBACK, blocking_bind, blocking_connect, blocking_listen,
    ip_socket_address_new, new_loopback, new_unspecified, port, tcp_socket_new,  };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, Network,
};

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_tcp_connect_unspec(net: &Network, family: IpAddressFamily) {
    let addr = ip_socket_address_new(new_unspecified(family), SOME_PORT);
    let sock = tcp_socket_new(family).unwrap();

    assert!(matches!(
        blocking_connect(&sock, net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_tcp_connect_port_0(net: &Network, family: IpAddressFamily) {
    let addr = ip_socket_address_new(new_loopback(family), 0);
    let sock = tcp_socket_new(family).unwrap();

    assert!(matches!(
        blocking_connect(&sock, net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should validate the address family.
fn test_tcp_connect_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IPV4_LOOPBACK,
    };
    let remote_addr = ip_socket_address_new(wrong_ip, SOME_PORT);

    let sock = tcp_socket_new(family).unwrap();

    assert!(matches!(
        blocking_connect(&sock, net, remote_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Can only connect to unicast addresses.
fn test_tcp_connect_non_unicast(net: &Network) {
    let ipv4_broadcast = ip_socket_address_new(IPV4_BROADCAST, SOME_PORT);
    let ipv4_multicast = ip_socket_address_new(IpAddress::Ipv4((224, 254, 0, 0)), SOME_PORT);
    let ipv6_multicast =
        ip_socket_address_new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), SOME_PORT);

    let sock_v4 = tcp_socket_new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = tcp_socket_new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(
        blocking_connect(&sock_v4, net, ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        blocking_connect(&sock_v4, net, ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        blocking_connect(&sock_v6, net, ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_connect_dual_stack(net: &Network) {
    // Set-up:
    let v4_listener = tcp_socket_new(IpAddressFamily::Ipv4).unwrap();
    blocking_bind(&v4_listener, &net, ip_socket_address_new(IPV4_LOOPBACK, 0))
        .unwrap();
    blocking_listen(&v4_listener).unwrap();

    let v4_listener_addr = v4_listener.local_address().unwrap();
    let v6_listener_addr =
        ip_socket_address_new(IPV4_MAPPED_LOOPBACK, port(&v4_listener_addr));

    let v6_client = tcp_socket_new(IpAddressFamily::Ipv6).unwrap();

    // Tests:

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert!(matches!(
        blocking_connect(&v6_client, net, v4_listener_addr),
        Err(ErrorCode::InvalidArgument)
    ));
    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert!(matches!(
        blocking_connect(&v6_client, net, v6_listener_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Client sockets can be explicitly bound.
fn test_tcp_connect_explicit_bind(net: &Network, family: IpAddressFamily) {
    let ip = new_loopback(family);

    let listener = {
        let bind_address = ip_socket_address_new(ip, 0);
        let listener = tcp_socket_new(family).unwrap();
        blocking_bind(&listener, &net, bind_address).unwrap();
        blocking_listen(&listener).unwrap();
        listener
    };

    let listener_address = listener.local_address().unwrap();
    let client = tcp_socket_new(family).unwrap();

    // Manually bind the client:
    blocking_bind(&client, net, ip_socket_address_new(ip, 0))
        .unwrap();

    // Connect should work:
    blocking_connect(&client, net, listener_address).unwrap();
}

fn main() {
    let net = instance_network();

    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_non_unicast(&net);

    test_tcp_connect_dual_stack(&net);

    test_tcp_connect_explicit_bind(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_explicit_bind(&net, IpAddressFamily::Ipv6);
}
