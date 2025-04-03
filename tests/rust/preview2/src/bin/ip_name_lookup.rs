use wasi::clocks::monotonic_clock;
use wasi::io::poll::{self, Pollable};
use wasi::sockets::instance_network::instance_network;
use wasi::sockets::network::{ErrorCode, IpAddress};

pub const IPV4_BROADCAST: IpAddress = IpAddress::Ipv4((255, 255, 255, 255));

pub const IPV4_LOOPBACK: IpAddress = IpAddress::Ipv4((127, 0, 0, 1));
pub const IPV6_LOOPBACK: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 1));

pub const IPV4_UNSPECIFIED: IpAddress = IpAddress::Ipv4((0, 0, 0, 0));
pub const IPV6_UNSPECIFIED: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 0));

// Workaround for orphan rule
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

fn main() {
    // Valid domains
    resolve("localhost").unwrap();
    resolve("example.com").unwrap();

    // NB: this is an actual real resolution, so it might time out, might cause
    // issues, etc. This result is ignored to prevent flaky failures in CI.
    let _ = resolve("münchen.de");

    // Valid IP addresses
    assert_eq!(IpAddressEq(resolve_one("0.0.0.0").unwrap()), IpAddressEq(IPV4_UNSPECIFIED));
    assert_eq!(IpAddressEq(resolve_one("127.0.0.1").unwrap()), IpAddressEq(IPV4_LOOPBACK));
    assert_eq!(
        IpAddressEq(resolve_one("192.0.2.0").unwrap()),
        IpAddressEq(IpAddress::Ipv4((192, 0, 2, 0)))
    );
    assert_eq!(IpAddressEq(resolve_one("::").unwrap()), IpAddressEq(IPV6_UNSPECIFIED));
    assert_eq!(IpAddressEq(resolve_one("::1").unwrap()), IpAddressEq(IPV6_LOOPBACK));
    assert_eq!(IpAddressEq(resolve_one("[::]").unwrap()), IpAddressEq(IPV6_UNSPECIFIED));
    assert_eq!(
        IpAddressEq(resolve_one("2001:0db8:0:0:0:0:0:0").unwrap()),
        IpAddressEq(IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)))
    );
    assert_eq!(
        IpAddressEq(resolve_one("dead:beef::").unwrap()),
        IpAddressEq(IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)))
    );
    assert_eq!(
        IpAddressEq(resolve_one("dead:beef::0").unwrap()),
        IpAddressEq(IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)))
    );
    assert_eq!(
        IpAddressEq(resolve_one("DEAD:BEEF::0").unwrap()),
        IpAddressEq(IpAddress::Ipv6((0xdead, 0xbeef, 0, 0, 0, 0, 0, 0)))
    );

    // Invalid inputs
    assert_eq!(resolve("").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(resolve(" ").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(resolve("a.b<&>").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(
        resolve("127.0.0.1:80").unwrap_err(),
        ErrorCode::InvalidArgument
    );
    assert_eq!(resolve("[::]:80").unwrap_err(), ErrorCode::InvalidArgument);
    assert_eq!(
        resolve("http://example.com/").unwrap_err(),
        ErrorCode::InvalidArgument
    );
}

fn resolve(name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
    blocking_resolve_addresses(name)
}

fn resolve_one(name: &str) -> Result<IpAddress, ErrorCode> {
    Ok(resolve(name)?.first().unwrap().to_owned())
}

pub fn blocking_resolve_addresses(name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
    let stream = wasi::sockets::ip_name_lookup::resolve_addresses(&instance_network(), name)?;

    let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
    let pollable = stream.subscribe();

    let mut addresses = vec![];

    loop {
        match stream.resolve_next_address() {
            Ok(Some(addr)) => {
                addresses.push(addr);
            }
            Ok(None) => match addresses[..] {
                [] => return Err(ErrorCode::NameUnresolvable),
                _ => return Ok(addresses),
            },
            Err(ErrorCode::WouldBlock) => {
                block_until(&pollable, &timeout)?;
            }
            Err(err) => return Err(err),
        }
    }
}

const TIMEOUT_NS: u64 = 5_000_000_000;

pub fn block_until(p: &Pollable, timeout: &Pollable) -> Result<(), ErrorCode> {
    let ready = poll::poll(&[p, timeout]);
    assert!(ready.len() > 0);
    match ready[0] {
        0 => Ok(()),
        1 => Err(ErrorCode::Timeout),
        _ => unreachable!(),
    }
}
