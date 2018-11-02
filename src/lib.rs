#[macro_use]
extern crate crossbeam;

use crossbeam::channel;
use crossbeam::queue::MsQueue;
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::string::String;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub mod resolvers;

pub trait Resolver {
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error>;
}

#[derive(Debug)]
pub struct Error(String);

pub struct ResolverPool<R> {
    resolver: Arc<R>,
    // Refresh interval in seconds.
    refresh_interval: Duration,
    state: State,
    stopc: channel::Sender<usize>,

    // Using a simple queue for now. In the future, maybe have this queue hold more than just a
    // SocketAddr, like current latency/error% and de-emphasize based on that. Maybe have a
    // "timeout" for bad SocketAddr's, and re-try them after a configurable timeout. e.g., the last
    // X connections have failed, or the avg latency for the last Y connections is above a
    // threshold, put it into a future that will add it back to the queue after Z milliseconds.
    cache: Arc<MsQueue<SocketAddr>>,
}

enum State {
    Initialized,
    Running,
}

impl<R> ResolverPool<R>
where
    R: 'static + Resolver + Send + Sync,
{
    pub fn new(resolver: R, refresh_interval: Duration) -> Self {
        let (stopc, _) = channel::bounded(0);
        ResolverPool {
            resolver: Arc::new(resolver),
            refresh_interval: refresh_interval,
            state: State::Initialized,
            stopc: stopc,
            cache: Arc::new(MsQueue::new()),
        }
    }

    pub fn get(&mut self) -> Option<SocketAddr> {
        let addr = self.cache.try_pop()?;
        self.cache.push(addr);
        Some(addr)
    }

    pub fn run(&mut self) -> Result<(), Error> {
        match self.state {
            State::Running => return Err(Error("already running".to_owned())),
            State::Initialized => {}
        };
        self.state = State::Running;

        while !self.cache.is_empty() {
            self.cache.pop();
        }

        for addr in self.resolver.resolve()? {
            self.cache.push(addr);
        }

        let (stopc, r) = channel::bounded(0);
        self.stopc = stopc;
        let refreshc = channel::tick(self.refresh_interval);
        let resolver = Arc::clone(&self.resolver);
        let cache = Arc::clone(&self.cache);
        thread::spawn(move || loop {
            select!{
                recv(refreshc) => {
                    match resolver.resolve() {
                        Ok(addrs) => {
                            if addrs.is_empty() {
                                continue
                            }
                            while !cache.is_empty() {
                                cache.pop();
                            }
                            for addr in addrs {
                                println!("adding addr {}", addr);
                                cache.push(addr);
                            }
                        }
                        Err(err) => println!("failed to refresh: {:?}", err),
                    }
                },
                recv(r) => return
            }
        });
        Ok(())
    }

    pub fn stop(self) {
        drop(self.stopc)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Resolver, ResolverPool};
    use std::net::SocketAddr;
    use std::time::Duration;

    struct DummyResolver(i8);
    impl DummyResolver {
        fn new(n: i8) -> Self {
            DummyResolver(n)
        }
    }
    impl Resolver for DummyResolver {
        fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
            let mut addrs = vec![];
            for n in 0..self.0 {
                let addr = format!("127.0.0.1:808{}", n).parse().unwrap();
                addrs.push(addr);
            }
            Ok(addrs)
        }
    }
    #[test]
    fn it_works() {
        let n = 5;
        let resolver = DummyResolver::new(n);
        let duration = Duration::new(5, 0); // 5 seconds
        let mut resolver_pool = ResolverPool::new(resolver, duration);
        assert_eq!(resolver_pool.run().is_ok(), true);
        let addr = resolver_pool.get();
        assert_eq!(addr.is_some(), true);
        let addr = addr.unwrap();
        assert_eq!(addr, "127.0.0.1:8080".parse().unwrap());
        for _n in 0..n * 2 {
            let addr = resolver_pool.get();
            assert_eq!(addr.is_some(), true);
        }
    }
}
