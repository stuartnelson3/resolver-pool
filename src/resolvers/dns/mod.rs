extern crate trust_dns;

use resolvers::dns::trust_dns::client::Client;
use resolvers::dns::trust_dns::op::DnsResponse;
use resolvers::dns::trust_dns::rr::{DNSClass, Name, RData, Record, RecordType};
use std::net::SocketAddr;
use std::str::FromStr;
use Error;
use Resolver;

pub struct TrustDNS<T> {
    client: T,
    lookup: &'static str,
}
// Make a generic ResolveStruct that can take a closure which returns the result of resolve().

impl<T> TrustDNS<T>
where
    T: Client,
{
    fn new(client: T, lookup: &'static str) -> Self {
        TrustDNS { client, lookup }
    }
}

impl<T> Resolver for TrustDNS<T>
where
    T: Client,
{
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        let name = Name::from_str(self.lookup).unwrap();
        // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        let response: DnsResponse = self
            .client
            .query(&name, DNSClass::IN, RecordType::SRV)
            .unwrap(); // Make record type configurable?
                       // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        let answers: &[Record] = response.answers();

        let srv_records: Vec<&RData> = answers.iter().map(|a| a.rdata()).collect();

        srv_records
            .iter()
            .filter_map(|srv| match srv {
                &RData::SRV(ref srv) => {
                    let ip = srv.target()[0].to_utf8();
                    let ip = ip.replace("-", "."); // TODO: Don't assume first target is ip addr.
                    Some(format!("{}:{}", ip, srv.port()))
                }
                _ => None,
            }).map(|ip| ip.parse().map_err(|_| Error("".to_string())))
            .collect()
    }
}
