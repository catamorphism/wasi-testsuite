use wasi_tests::tcp_helpers::{ IPV4_LOOPBACK, IPV6_LOOPBACK, IPV4_MAPPED_LOOPBACK,
    IpSocketAddressEq, blocking_bind_udp, blocking_bind_unspecified,
    ip_socket_address_new, new_loopback, new_unspecified,
    port, udp_socket_new };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddressFamily, Network,
};

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

fn test_udp_connect_disconnect_reconnect(net: &Network, family: IpAddressFamily) {
    let unspecified_addr = ip_socket_address_new(new_unspecified(family), 0);
    let remote1 = ip_socket_address_new(new_loopback(family), 4321);
    let remote2 = ip_socket_address_new(new_loopback(family), 4320);

    let client = udp_socket_new(family).unwrap();
    blocking_bind_udp(&client, &net, unspecified_addr).unwrap();

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address().err(), Some(ErrorCode::InvalidState));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address().err(), Some(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(IpSocketAddressEq(client.remote_address().unwrap()), IpSocketAddressEq(remote1));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(IpSocketAddressEq(client.remote_address().unwrap()), IpSocketAddressEq(remote1));

    _ = client.stream(Some(remote2)).unwrap();
    assert_eq!(IpSocketAddressEq(client.remote_address().unwrap()), IpSocketAddressEq(remote2));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address().err(), Some(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(IpSocketAddressEq(client.remote_address().unwrap()), IpSocketAddressEq(remote1));
}

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_udp_connect_unspec(net: &Network, family: IpAddressFamily) {
    let addr = ip_socket_address_new(new_unspecified(family), SOME_PORT);
    let sock = udp_socket_new(family).unwrap();
    blocking_bind_unspecified(&sock, &net).unwrap();

    assert!(matches!(
        sock.stream(Some(addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_udp_connect_port_0(net: &Network, family: IpAddressFamily) {
    let addr = ip_socket_address_new(new_loopback(family), 0);
    let sock = udp_socket_new(family).unwrap();
    blocking_bind_unspecified(&sock, &net).unwrap();

    assert!(matches!(
        sock.stream(Some(addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should validate the address family.
fn test_udp_connect_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IPV4_LOOPBACK,
    };
    let remote_addr = ip_socket_address_new(wrong_ip, SOME_PORT);

    let sock = udp_socket_new(family).unwrap();
    blocking_bind_unspecified(&sock, &net).unwrap();

    assert!(matches!(
        sock.stream(Some(remote_addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_udp_connect_dual_stack(net: &Network) {
    // Set-up:
    let v4_server = udp_socket_new(IpAddressFamily::Ipv4).unwrap();
    blocking_bind_udp(&v4_server, &net, ip_socket_address_new(IPV4_LOOPBACK, 0))
        .unwrap();

    let v4_server_addr = v4_server.local_address().unwrap();
    let v6_server_addr =
        ip_socket_address_new(IPV4_MAPPED_LOOPBACK, port(&v4_server_addr));

    // Tests:
    let v6_client = udp_socket_new(IpAddressFamily::Ipv6).unwrap();

    blocking_bind_unspecified(&v6_client, &net).unwrap();

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.stream(Some(v4_server_addr)),
        Err(ErrorCode::InvalidArgument)
    ));

    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.stream(Some(v6_server_addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn main() {
    let net = instance_network();

    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv4);
    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv6);

    test_udp_connect_unspec(&net, IpAddressFamily::Ipv4);
    test_udp_connect_unspec(&net, IpAddressFamily::Ipv6);

    test_udp_connect_port_0(&net, IpAddressFamily::Ipv4);
    test_udp_connect_port_0(&net, IpAddressFamily::Ipv6);

    test_udp_connect_wrong_family(&net, IpAddressFamily::Ipv4);
    test_udp_connect_wrong_family(&net, IpAddressFamily::Ipv6);

    test_udp_connect_dual_stack(&net);
}
