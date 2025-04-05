use wasi_tests::tcp_helpers::{ IpSocketAddressEq, blocking_bind_udp, blocking_receive,
    blocking_send, ip_socket_address_new, new_unspecified, udp_socket_new };
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};
use wasi::sockets::udp::{OutgoingDatagram};

fn test_udp_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let unspecified_addr = ip_socket_address_new(new_unspecified(family), 0);

    let first_message = &[];
    let second_message = b"Hello, world!";
    let third_message = b"Greetings, planet!";

    let net = instance_network();

    let server = udp_socket_new(family).unwrap();

    blocking_bind_udp(&server, &net, bind_address).unwrap();
    let (server_incoming, _) = server.stream(None).unwrap();
    let addr = server.local_address().unwrap();

    let client_addr = {
        let client = udp_socket_new(family).unwrap();
        blocking_bind_udp(&client, &net, unspecified_addr).unwrap();
        let (_, client_outgoing) = client.stream(Some(addr)).unwrap();

        let datagrams = [
            OutgoingDatagram {
                data: first_message.to_vec(),
                remote_address: None,
            },
            OutgoingDatagram {
                data: second_message.to_vec(),
                remote_address: Some(addr),
            },
        ];
        blocking_send(&client_outgoing, &datagrams).unwrap();

        client.local_address().unwrap()
    };

    {
        // Check that we've received our sent messages.
        let datagrams = blocking_receive(&server_incoming, 2..100).unwrap();
        assert_eq!(datagrams.len(), 2);

        assert_eq!(datagrams[0].data, first_message);
        assert_eq!(IpSocketAddressEq(datagrams[0].remote_address), IpSocketAddressEq(client_addr));

        assert_eq!(datagrams[1].data, second_message);
        assert_eq!(IpSocketAddressEq(datagrams[1].remote_address), IpSocketAddressEq(client_addr));
    }

    // Another client
    {
        let client = udp_socket_new(family).unwrap();
        blocking_bind_udp(&client, &net, unspecified_addr).unwrap();
        let (_, client_outgoing) = client.stream(None).unwrap();

        let datagrams = [OutgoingDatagram {
            data: third_message.to_vec(),
            remote_address: Some(addr),
        }];
        blocking_send(&client_outgoing, &datagrams).unwrap();
    }

    {
        // Check that we sent and received our message!
        let datagrams = blocking_receive(&server_incoming, 1..100).unwrap();
        assert_eq!(datagrams.len(), 1);

        assert_eq!(datagrams[0].data, third_message);
    }
}

fn main() {
    test_udp_sample_application(
        IpAddressFamily::Ipv4,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,                 // use any free port
            address: (127, 0, 0, 1), // localhost
        }),
    );
    test_udp_sample_application(
        IpAddressFamily::Ipv6,
        IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port: 0,                           // use any free port
            address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
            flow_info: 0,
            scope_id: 0,
        }),
    );
}
