#[macro_use]
extern crate crossbeam;

use crossbeam::channel;
use crossbeam::queue::MsQueue;
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::string::String;
use std::thread;
use std::time::Duration;

pub trait Resolver {
    fn resolve<'a>(&self, key: &'a str) -> Result<Vec<SocketAddr>, Error>;
}

#[derive(Debug)]
pub struct Error(String);

pub struct ResolverPool<R> {
    resolver: R,
    // Refresh interval in seconds.
    refresh_interval: Duration,
    state: State,
    stopc: channel::Sender<usize>,

    // Using a simple queue for now. In the future, maybe have this queue hold more than just a
    // SocketAddr, like current latency/error% and de-emphasize based on that. Maybe have a
    // "timeout" for bad SocketAddr's, and re-try them after a configurable timeout. e.g., the last
    // X connections have failed, or the avg latency for the last Y connections is above a
    // threshold, put it into a future that will add it back to the queue after Z milliseconds.
    cache: MsQueue<SocketAddr>,
}

enum State {
    Initialized,
    Running,
}

impl<R> ResolverPool<R>
where
    R: Resolver + Send + Sync,
{
    pub fn new(resolver: R, refresh_interval: Duration) -> Self {
        let (stopc, _) = channel::bounded(0);
        ResolverPool {
            resolver: resolver,
            refresh_interval: refresh_interval,
            state: State::Initialized,
            stopc: stopc,
            cache: MsQueue::new(),
        }
    }

    pub fn get(&mut self) -> Option<SocketAddr> {
        let addr = self.cache.try_pop();
        if let Some(addr) = addr {
            self.cache.push(addr);
        }
        addr
    }

    pub fn run<'a>(&mut self, key: &'a str) -> Result<(), Error> {
        match self.state {
            State::Running => return Err(Error("already running".to_owned())),
            State::Initialized => {}
        };
        self.state = State::Running;

        for addr in self.resolver.resolve(key)? {
            self.cache.push(addr);
        }

        let (stopc, r) = channel::bounded(0);
        self.stopc = stopc;
        let refreshc = channel::tick(self.refresh_interval);
        thread::spawn(move || loop {
            select!{
                recv(refreshc) => {
                    // match self.resolver.resolve(key) {
                        // Ok(addrs) => {
                        // if addrs.is_empty() {
                        //     continue
                        // }
                        // while !self.cache.is_empty() {
                        //     self.cache.pop();
                        // }
                        // for addr in addrs {
                        //     self.cache.push(addr);
                        // }
                        // Err(err) => print!("failed to refresh: {:?}", err),
                    // }
                },
                recv(r.clone()) => return
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
