extern crate trust_dns;

use rayon::prelude::*;
use resolvers::dns::trust_dns::client::Client;
use resolvers::dns::trust_dns::client::SyncClient;
use resolvers::dns::trust_dns::op::DnsResponse;
use resolvers::dns::trust_dns::rr::{DNSClass, Name, RData, Record, RecordType};
use resolvers::dns::trust_dns::tcp::TcpClientConnection;
use resolvers::dns::trust_dns::udp::UdpClientConnection;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use Error;
use Resolver;

pub struct ParallelResolver {
    pub clients: Vec<TrustDNS>,
}

impl ParallelResolver {
    // TODO: Inject Prometheus registry.
    pub fn new(nameservers: Vec<SocketAddr>, lookup: &'static str) -> Self {
        let clients = nameservers
            .into_iter()
            .map(|ns| TrustDNS {
                udp: SyncClient::new(UdpClientConnection::new(ns).unwrap()),
                tcp: SyncClient::new(TcpClientConnection::new(ns).unwrap()),
                lookup: lookup,
            }).collect();
        ParallelResolver { clients }
    }
}

impl Resolver<SocketAddr> for ParallelResolver {
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        self.clients
            .par_iter()
            .map(|client| client.resolve())
            .find_any(|res| res.is_ok())
            .unwrap_or(Err(Error("request failed".to_string())))
    }
}

pub struct TrustDNS {
    pub udp: SyncClient<UdpClientConnection>,
    pub tcp: SyncClient<TcpClientConnection>,
    pub lookup: &'static str,
}

impl Resolver<SocketAddr> for TrustDNS {
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        let name = Name::from_str(self.lookup).map_err(|err| Error(err.to_string()))?;

        let mut response: DnsResponse = match self.udp.query(&name, DNSClass::IN, RecordType::SRV) {
            Ok(response) => response,
            Err(err) => {
                error!("{}", err.to_string());
                return Err(Error(err.to_string()));
            }
        };

        if response.truncated() {
            response = match self.tcp.query(&name, DNSClass::IN, RecordType::SRV) {
                Ok(response) => response,
                Err(err) => {
                    error!("{}", err.to_string());
                    return Err(Error(err.to_string()));
                }
            };
        }

        let srv_records = response
            .answers()
            .par_iter()
            .map(|a| a.rdata())
            .filter_map(|srv| match srv {
                &RData::SRV(ref srv) => {
                    let mut response: DnsResponse = self
                        .udp
                        .query(&srv.target(), DNSClass::IN, RecordType::A)
                        .ok()?;

                    if response.truncated() {
                        response = self
                            .tcp
                            .query(&srv.target(), DNSClass::IN, RecordType::A)
                            .ok()?;
                    }

                    let answer: &Record = response.answers().first()?;
                    if let &RData::A(ip) = answer.rdata() {
                        Some(SocketAddr::new(IpAddr::V4(ip), srv.port()))
                    } else {
                        error!(
                            "rdata did not contain an A type record: {:?}",
                            answer.rdata()
                        );
                        None
                    }
                }
                _ => {
                    error!("rdata did not contain an SRV record type");
                    None
                }
            }).collect();
        Ok(srv_records)
    }
}
