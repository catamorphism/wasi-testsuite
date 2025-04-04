use wasi_tests::tcp_helpers::{ IPV4_BROADCAST, IPV4_LOOPBACK, IPV4_MAPPED_LOOPBACK,
    IPV4_UNSPECIFIED, IPV6_LOOPBACK, IPV6_UNSPECIFIED, IpAddressEq,
    attempt_random_port,
    blocking_accept, blocking_bind,
    blocking_connect, blocking_listen, ip_family, ip_socket_address_ip,
    ip_socket_address_new, new_loopback, port, tcp_socket_new,  };

use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily,  Network,
};

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
