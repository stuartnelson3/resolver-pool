#[macro_use]
extern crate crossbeam;

use crossbeam::channel;
use std::net::SocketAddr;
use std::string::String;
use std::thread;
use std::time::Duration;

pub trait Resolver {
    fn resolve<'a>(key: &'a str) -> Result<Vec<SocketAddr>, Error>;
}

pub struct Error(String);

pub struct ResolverPool<R> {
    resolver: R,
    // Refresh interval in seconds.
    refresh_interval: Duration,
    state: State,
    stopc: channel::Sender<usize>,
}

enum State {
    Initialized,
    Running,
}

impl<R> ResolverPool<R>
where
    R: Resolver,
{
    pub fn new(resolver: R, refresh_interval: Duration) -> Self {
        let (stopc, _) = channel::bounded(0);
        ResolverPool {
            resolver: resolver,
            refresh_interval: refresh_interval,
            state: State::Initialized,
            stopc: stopc,
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        match self.state {
            State::Running => return Err(Error("already running".to_owned())),
            State::Initialized => {}
        };
        self.state = State::Running;

        let (stopc, r) = channel::bounded(0);
        self.stopc = stopc;
        let refreshc = channel::tick(self.refresh_interval);
        thread::spawn(move || loop {
            select!{
                recv(refreshc) => {
                },
                recv(r) => return
            }
        });
        Ok(())
    }

    pub fn stop(mut self) {
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
