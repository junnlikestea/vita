#![allow(clippy::rc_buffer)]

pub mod crobat {
    include!("../proto/pb/proto.rs");
}

extern crate pretty_env_logger;
#[macro_use]
extern crate log;
use crobat::crobat_client::CrobatClient;
use crobat::Domain;
use crobat::QueryRequest;
use futures_core::stream::Stream;
use std::sync::Arc;
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Status;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct Crobat {
    client: CrobatClient<Channel>,
}

impl Crobat {
    pub async fn connect() -> Result<Self> {
        trace!("building crobat client");
        let addr = "https://crobat-rpc.omnisint.io";
        match Crobat::build_tls_client(addr).await {
            Ok(conn) => Ok(Self {
                client: CrobatClient::new(conn),
            }),
            Err(e) => Err(e),
        }
    }

    async fn build_tls_client(url: &'static str) -> Result<Channel> {
        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        config.alpn_protocols = vec![b"h2".to_vec()];

        let conn = Channel::from_static(url)
            .tls_config(ClientTlsConfig::new().rustls_client_config(config))?
            .connect()
            .await?;

        Ok(conn)
    }

    pub async fn get_subs(
        &mut self,
        host: Arc<String>,
    ) -> Result<impl Stream<Item = std::result::Result<Domain, Status>>> {
        trace!("querying crobat client for subdomains");
        let request = tonic::Request::new(QueryRequest {
            query: host.to_string(),
        });
        let stream = self.client.get_subdomains(request).await?.into_inner();

        Ok(stream)
    }
}
