extern crate env_logger;
extern crate resolver_pool;
extern crate trust_dns;
#[macro_use]
extern crate log;

use resolver_pool::resolvers::dns;
use resolver_pool::ResolverPool;
use std::thread;
use std::time::Duration;

pub fn main() {
    env_logger::init();
    let address = "0.0.0.0:1053".parse().unwrap();

    let srv_record = "_proto._service.example.com";
    let resolver = dns::TrustDNS::new(address, srv_record);

    let micro_second = 1 * 1000;
    let milli_second = micro_second * 1000;
    let duration = Duration::new(1, 0); // 5 seconds
    let mut resolver_pool = ResolverPool::new(resolver, duration);
    resolver_pool.run().expect("pool failed to run");
    for _ in 0..4 {
        for _ in 0..2000 {
            match resolver_pool.get() {
                Some(addr) => info!("got addr: {}", addr),
                None => {
                    error!("failed to get addr!");
                    return;
                }
            }
            thread::sleep(Duration::new(0, 10 * milli_second));
        }
    }
}
