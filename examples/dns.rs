extern crate env_logger;
extern crate resolver_pool;
extern crate trust_dns;
#[macro_use]
extern crate log;
extern crate trust_dns_resolver;

use resolver_pool::resolvers::dns;
use resolver_pool::ResolverPool;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use trust_dns_resolver::system_conf::read_system_conf;

pub fn main() {
    env_logger::init();
    let (config, _) = read_system_conf().unwrap();
    let mut addresses = config
        .name_servers()
        .iter()
        .map(|ns| ns.socket_addr)
        .collect::<Vec<SocketAddr>>();
    addresses.sort_by(|a, b| a.ip().cmp(&b.ip()));
    addresses.dedup();

    let srv_record = "_proto._service.example.com.";
    let resolver = dns::ParallelResolver::new(addresses, srv_record);

    let duration = Duration::new(5, 0);
    let mut resolver_pool = ResolverPool::new(resolver, duration);
    resolver_pool.run().expect("pool failed to run");
    for _ in 0..10 {
        match resolver_pool.get() {
            Some(addr) => info!("got addr: {}", addr),
            None => info!("nothing"),
        }
        thread::sleep(Duration::new(1, 0));
    }
}
