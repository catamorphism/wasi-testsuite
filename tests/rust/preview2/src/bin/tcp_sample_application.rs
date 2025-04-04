use wasi_tests::tcp_helpers::{  blocking_accept, blocking_bind, blocking_connect, blocking_listen,
    blocking_write_util, tcp_socket_new, };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};

fn test_tcp_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let net = instance_network();
    let listener = tcp_socket_new(family).unwrap();

    blocking_bind(&listener, &net, bind_address).unwrap();
    listener.set_listen_backlog_size(32).unwrap();
    blocking_listen(&listener).unwrap();

    let addr = listener.local_address().unwrap();

    {
        let client = tcp_socket_new(family).unwrap();
        let (_client_input, client_output) = blocking_connect(&client, &net, addr).unwrap();

        blocking_write_util(&client_output, &[]).unwrap();
        blocking_write_util(&client_output, first_message).unwrap();
    }

    {
        let (_accepted, input, _output) = blocking_accept(&listener).unwrap();

        let empty_data = input.read(0).unwrap();
        assert!(empty_data.is_empty());

        let data = input.blocking_read(first_message.len() as u64).unwrap();

        // Check that we sent and received our message!
        assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.
    }

    // Another client
    {
        let client = tcp_socket_new(family).unwrap();
        let (_client_input, client_output) = blocking_connect(&client, &net, addr).unwrap();

        blocking_write_util(&client_output, second_message).unwrap();
    }

    {
        let (_accepted, input, _output) = blocking_accept(&listener).unwrap();
        let data = input.blocking_read(second_message.len() as u64).unwrap();

        // Check that we sent and received our message!
        assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.
    }
}

fn main() {
    test_tcp_sample_application(
        IpAddressFamily::Ipv4,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,                 // use any free port
            address: (127, 0, 0, 1), // localhost
        }),
    );
    test_tcp_sample_application(
        IpAddressFamily::Ipv6,
        IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port: 0,                           // use any free port
            address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
            flow_info: 0,
            scope_id: 0,
        }),
    );
}
