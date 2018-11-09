extern crate trust_dns;

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

pub struct TrustDNS {
    pub udp_client: SyncClient<UdpClientConnection>,
    pub tcp_client: SyncClient<TcpClientConnection>,
    pub lookup: &'static str,
}

impl TrustDNS {
    // TODO: Inject Prometheus registry.
    // TODO: Support multiple nameservers
    pub fn new(name_server: SocketAddr, lookup: &'static str) -> Self {
        let udp_client = SyncClient::new(UdpClientConnection::new(name_server).unwrap());
        let tcp_client = SyncClient::new(TcpClientConnection::new(name_server).unwrap());
        TrustDNS {
            udp_client,
            tcp_client,
            lookup,
        }
    }
}

impl Resolver<SocketAddr> for TrustDNS {
    fn resolve(&self) -> Result<Vec<SocketAddr>, Error> {
        let name = Name::from_str(self.lookup).expect("failed to create name");
        // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        let mut response: DnsResponse = self
            .udp_client
            .query(&name, DNSClass::IN, RecordType::SRV)
            .expect("failed to query client");
        // .map_err(|err| Err(Error("TODO: Map error".to_string())))?;

        if response.truncated() {
            response = self
                .tcp_client
                .query(&name, DNSClass::IN, RecordType::SRV)
                .expect("failed to query client");
        }

        let srv_records = response
            .answers()
            .iter()
            .map(|a| a.rdata())
            .filter_map(|srv| match srv {
                &RData::SRV(ref srv) => {
                    let mut response: DnsResponse = self
                        .udp_client
                        .query(&srv.target(), DNSClass::IN, RecordType::A)
                        .ok()?;
                    // Log failure
                    // .expect("A record lookup failed");

                    if response.truncated() {
                        response = self
                            .udp_client
                            .query(&srv.target(), DNSClass::IN, RecordType::A)
                            .ok()?;
                    }

                    let answer: &Record = response.answers().first()?;
                    // Log failure
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
                _ => None,
            }).collect();
        Ok(srv_records)
    }
}
