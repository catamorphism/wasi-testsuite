use wasi_tests::tcp_helpers::{ IPV4_LOOPBACK, IPV6_LOOPBACK, IPV4_MAPPED_LOOPBACK,
    IPV4_UNSPECIFIED, IPV6_UNSPECIFIED, IpAddressEq,
    attempt_random_port, blocking_bind_udp, ip_family,
    ip_socket_address_ip, ip_socket_address_new, port, udp_socket_new };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, Network,
};

/// Bind a socket and let the system determine a port.
fn test_udp_bind_ephemeral_port(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock = udp_socket_new(ip_family(&ip)).unwrap();
    blocking_bind_udp(&sock, net, bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(IpAddressEq(ip_socket_address_ip(&bind_addr)), IpAddressEq(ip_socket_address_ip(&bound_addr)));
    assert_ne!(port(&bind_addr), port(&bound_addr));
}

/// Bind a socket on a specified port.
fn test_udp_bind_specific_port(net: &Network, ip: IpAddress) {
    let sock = udp_socket_new(ip_family(&ip)).unwrap();

    let bind_addr =
        attempt_random_port(ip, |bind_addr| blocking_bind_udp(&sock, net, bind_addr)).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(IpAddressEq(ip_socket_address_ip(&bind_addr)), IpAddressEq(ip_socket_address_ip(&bound_addr)));
    assert_eq!(port(&bind_addr), port(&bound_addr));
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_udp_bind_addrinuse(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock1 = udp_socket_new(ip_family(&ip)).unwrap();
    blocking_bind_udp(&sock1, net, bind_addr).unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = udp_socket_new(ip_family(&ip)).unwrap();
    assert!(matches!(
        blocking_bind_udp(&sock2, net, bound_addr),
        Err(ErrorCode::AddressInUse)
    ));
}

// Try binding to an address that is not configured on the system.
fn test_udp_bind_addrnotavail(net: &Network, ip: IpAddress) {
    let bind_addr = ip_socket_address_new(ip, 0);

    let sock = udp_socket_new(ip_family(&ip)).unwrap();

    assert!(matches!(
        blocking_bind_udp(&sock, net, bind_addr),
        Err(ErrorCode::AddressNotBindable)
    ));
}

/// Bind should validate the address family.
fn test_udp_bind_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IPV4_LOOPBACK,
    };

    let sock = udp_socket_new(family).unwrap();
    let result = blocking_bind_udp(&sock, net, ip_socket_address_new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

fn test_udp_bind_dual_stack(net: &Network) {
    let sock = udp_socket_new(IpAddressFamily::Ipv6).unwrap();
    let addr = ip_socket_address_new(IPV4_MAPPED_LOOPBACK, 0);

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(
        blocking_bind_udp(&sock, net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn main() {
    const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
    const RESERVED_IPV6_ADDRESS: IpAddress = IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

    let net = instance_network();

    test_udp_bind_ephemeral_port(&net, IPV4_LOOPBACK);
    test_udp_bind_ephemeral_port(&net, IPV6_LOOPBACK);
    test_udp_bind_ephemeral_port(&net, IPV4_UNSPECIFIED);
    test_udp_bind_ephemeral_port(&net, IPV6_UNSPECIFIED);

    test_udp_bind_specific_port(&net, IPV4_LOOPBACK);
    test_udp_bind_specific_port(&net, IPV6_LOOPBACK);
    test_udp_bind_specific_port(&net, IPV4_UNSPECIFIED);
    test_udp_bind_specific_port(&net, IPV6_UNSPECIFIED);

    test_udp_bind_addrinuse(&net, IPV4_LOOPBACK);
    test_udp_bind_addrinuse(&net, IPV6_LOOPBACK);
    test_udp_bind_addrinuse(&net, IPV4_UNSPECIFIED);
    test_udp_bind_addrinuse(&net, IPV6_UNSPECIFIED);

    test_udp_bind_addrnotavail(&net, RESERVED_IPV4_ADDRESS);
    test_udp_bind_addrnotavail(&net, RESERVED_IPV6_ADDRESS);

    test_udp_bind_wrong_family(&net, IpAddressFamily::Ipv4);
    test_udp_bind_wrong_family(&net, IpAddressFamily::Ipv6);

    test_udp_bind_dual_stack(&net);
}
