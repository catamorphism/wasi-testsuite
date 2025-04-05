use wasi_tests::tcp_helpers::{ blocking_bind_udp, ip_socket_address_new, new_loopback,
    udp_socket_new };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddressFamily, Network,
};

fn test_udp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = udp_socket_new(family).unwrap();

    // Skipping: udp::start_bind
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));

    assert!(matches!(sock.stream(None), Err(ErrorCode::InvalidState)));

    assert!(matches!(sock.local_address(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_udp_bound_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = ip_socket_address_new(new_loopback(family), 0);
    let sock = udp_socket_new(family).unwrap();
    blocking_bind_udp(&sock, net, bind_address).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: udp::stream

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_udp_connected_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = ip_socket_address_new(new_loopback(family), 0);
    let connect_address = ip_socket_address_new(new_loopback(family), 54321);
    let sock = udp_socket_new(family).unwrap();
    blocking_bind_udp(&sock, net, bind_address).unwrap();
    sock.stream(Some(connect_address)).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: udp::stream

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(sock.remote_address(), Ok(_)));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn main() {
    let net = instance_network();

    test_udp_unbound_state_invariants(IpAddressFamily::Ipv4);
    test_udp_unbound_state_invariants(IpAddressFamily::Ipv6);

    test_udp_bound_state_invariants(&net, IpAddressFamily::Ipv4);
    test_udp_bound_state_invariants(&net, IpAddressFamily::Ipv6);

    test_udp_connected_state_invariants(&net, IpAddressFamily::Ipv4);
    test_udp_connected_state_invariants(&net, IpAddressFamily::Ipv6);
}
