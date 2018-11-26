#[macro_use]
extern crate crossbeam;
#[macro_use]
extern crate log;
extern crate rayon;

use crossbeam::channel;
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::string::String;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
    cache: Arc<Mutex<Vec<SocketAddr>>>,
    offset: AtomicUsize,
}

enum State {
    Initialized,
    Running,
}

impl<R> ResolverPool<R>
where
    R: 'static + Resolver + Send + Sync,
{
    // TODO: Inject Prometheus registry.
    pub fn new(resolver: R, refresh_interval: Duration) -> Self {
        let (stopc, _) = channel::bounded(0);
        ResolverPool {
            resolver: Arc::new(resolver),
            refresh_interval: refresh_interval,
            state: State::Initialized,
            stopc: stopc,
            cache: Arc::new(Mutex::new(vec![])),
            offset: AtomicUsize::new(0),
        }
    }

    pub fn get(&mut self) -> Option<SocketAddr> {
        let offset = self.offset.load(Ordering::Relaxed);
        self.offset.store(offset + 1, Ordering::Relaxed);
        let cache = self.cache.lock().unwrap();
        let len = cache.len();
        if len == 0 {
            return None;
        }
        Some(cache[offset % len])
    }

    pub fn run(&mut self) -> Result<(), Error> {
        match self.state {
            State::Running => {
                error!("resolver pool already running");
                return Err(Error("resolver pool already running".to_owned()));
            }
            State::Initialized => {}
        };
        self.state = State::Running;

        *self.cache.lock().unwrap() = self.resolver.resolve()?;

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
                            if addrs.len() > 0 {
                                *cache.lock().unwrap() = addrs;
                            } else {
                                debug!("no addrs returned from resolvers");
                            }
                        }
                        Err(err) => error!("failed to refresh: {:?}", err),
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
        let addr = resolver_pool.get().unwrap();
        assert_eq!(addr, "127.0.0.1:8080".parse().unwrap());
    }
}
