extern crate resolver_pool;
extern crate trust_dns;

use resolver_pool::resolvers::dns;
use resolver_pool::ResolverPool;
use std::thread;
use std::time::Duration;
use trust_dns::client::SyncClient;
use trust_dns::tcp::TcpClientConnection;

pub fn main() {
    let address = "0.0.0.0:1053".parse().unwrap();
    let conn = TcpClientConnection::new(address).unwrap();

    let client = SyncClient::new(conn);
    let srv_record = "_proto._service.example.com";
    let resolver = dns::TrustDNS::new(client, srv_record);

    let duration = Duration::new(5, 0); // 5 seconds
    let mut resolver_pool = ResolverPool::new(resolver, duration);
    resolver_pool.run().expect("pool failed to run");
    for _ in 0..4 {
        for _ in 0..20 {
            match resolver_pool.get() {
                Some(addr) => println!("got addr: {}", addr),
                None => println!("failed to get addr!"),
            }
        }
        thread::sleep(Duration::new(6, 0));
    }
}
