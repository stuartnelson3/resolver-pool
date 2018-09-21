use std::net::SocketAddr;
use std::string::String;
use std::time::Duration;

pub trait Resolver {
    fn resolve<'a>(key: &'a str) -> Result<Vec<SocketAddr>, Error>;
}

pub struct Error(String);

pub struct ResolverPool<R> {
    resolver: R,
    refresh_interval: Duration,
}

impl<R> ResolverPool<R>
where
    R: Resolver,
{
    pub fn new(resolver: R, refresh_interval: Duration) -> Self {
        ResolverPool {
            resolver: resolver,
            refresh_interval: refresh_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
