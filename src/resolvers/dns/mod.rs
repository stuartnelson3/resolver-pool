extern crate trust_dns;

use resolvers::dns::trust_dns::client::Client;
use resolvers::dns::trust_dns::op::DnsResponse;
use resolvers::dns::trust_dns::rr::{DNSClass, Name, RData, Record, RecordType};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use Error;
use Resolver;

pub struct TrustDNS<T> {
    client: T,
    lookup: &'static str,
}

impl<T> TrustDNS<T>
where
    T: Client,
{
    // TODO: Inject Prometheus registry.
    pub fn new(client: T, lookup: &'static str) -> Self {
        TrustDNS { client, lookup }
    }
}

impl<T> Resolver<SocketAddr> for TrustDNS<T>
where
    T: Client,
{
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        let name = Name::from_str(self.lookup).expect("failed to create name");
        // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        let response: DnsResponse = self
            .client
            .query(&name, DNSClass::IN, RecordType::SRV)
            .expect("failed to query client");
        // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        let answers: &[Record] = response.answers();

        let srv_records = answers
            .iter()
            .map(|a| a.rdata())
            .filter_map(|srv| match srv {
                &RData::SRV(ref srv) => {
                    let response: DnsResponse = self
                        .client
                        .query(&srv.target(), DNSClass::IN, RecordType::A)
                        .ok()?;
                    // Log failure
                    // .expect("A record lookup failed");
                    let answer: &Record = response.answers().first()?;
                    // Log failure
                    if let &RData::A(ip) = answer.rdata() {
                        Some(SocketAddr::new(IpAddr::V4(ip), srv.port()))
                    } else {
                        // Log failure.
                        None
                    }
                }
                _ => None,
            }).collect();
        Ok(srv_records)
    }
}
