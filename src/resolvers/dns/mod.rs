extern crate trust_dns;

use resolvers::dns::trust_dns::client::Client;
use std::net::SocketAddr;
use Error;
use Resolver;

pub struct TrustDNS<T> {
    client: T,
}
// Make a generic ResolveStruct that can take a closure which returns the result of resolve().

impl<T> TrustDNS<T>
where
    T: Client,
{
    fn new(client: T) -> Self {
        TrustDNS { client }
    }
}

impl<T> Resolver for TrustDNS<T>
where
    T: Client,
{
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        Err(Error("test compile".to_owned()))
    }
}
