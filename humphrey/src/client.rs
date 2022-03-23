//! Provides an HTTP client implementation for Humphrey.

use crate::http::address::Address;
use crate::http::headers::{RequestHeader, RequestHeaderMap};
use crate::http::method::Method;
use crate::http::{Request, Response};

use std::error::Error;
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

#[cfg(feature = "tls")]
use rustls::{Certificate, ClientConfig, ClientConnection, RootCertStore, StreamOwned};
#[cfg(feature = "tls")]
use rustls_native_certs::load_native_certs;
#[cfg(feature = "tls")]
use std::convert::TryInto;
#[cfg(feature = "tls")]
use std::sync::Arc;

/// Represents an HTTP client.
///
/// When TLS is enabled, this is fairly expensive to instantiate,
///   so should only be done once per program instead of once per request.
#[derive(Default)]
pub struct Client {
    #[cfg(feature = "tls")]
    tls_config: Option<Arc<ClientConfig>>,
}

impl Client {
    /// Creates a new HTTP client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a GET request to the given URL.
    pub fn get(&mut self, url: impl AsRef<str>) -> Result<ClientRequest, Box<dyn Error>> {
        let url = Self::parse_url(url).ok_or("Invalid URL")?;
        let request = Request {
            method: Method::Get,
            uri: url.path,
            headers: url.host_headers,
            query: url.query,
            version: "HTTP/1.1".to_string(),
            content: None,
            address: Address::new(url.host).unwrap(),
        };

        Ok(ClientRequest {
            address: url.host,
            client: self,
            protocol: url.protocol,
            request,
        })
    }

    /// Creates a POST request to the given URL.
    pub fn post(
        &mut self,
        url: impl AsRef<str>,
        data: Vec<u8>,
    ) -> Result<ClientRequest, Box<dyn Error>> {
        let url = Self::parse_url(url).ok_or("Invalid URL")?;
        let request = Request {
            method: Method::Post,
            uri: url.path,
            headers: url.host_headers,
            query: url.query,
            version: "HTTP/1.1".to_string(),
            content: Some(data),
            address: Address::new(url.host).unwrap(),
        };

        Ok(ClientRequest {
            address: url.host,
            client: self,
            protocol: url.protocol,
            request,
        })
    }

    /// Creates a PUT request to the given URL.
    pub fn put(
        &mut self,
        url: impl AsRef<str>,
        data: Vec<u8>,
    ) -> Result<ClientRequest, Box<dyn Error>> {
        let url = Self::parse_url(url).ok_or("Invalid URL")?;
        let request = Request {
            method: Method::Put,
            uri: url.path,
            headers: url.host_headers,
            query: url.query,
            version: "HTTP/1.1".to_string(),
            content: Some(data),
            address: Address::new(url.host).unwrap(),
        };

        Ok(ClientRequest {
            address: url.host,
            client: self,
            protocol: url.protocol,
            request,
        })
    }

    /// Creates a DELETE request to the given URL.
    pub fn delete(&mut self, url: impl AsRef<str>) -> Result<ClientRequest, Box<dyn Error>> {
        let url = Self::parse_url(url).ok_or("Invalid URL")?;
        let request = Request {
            method: Method::Delete,
            uri: url.path,
            headers: url.host_headers,
            query: url.query,
            version: "HTTP/1.1".to_string(),
            content: None,
            address: Address::new(url.host).unwrap(),
        };

        Ok(ClientRequest {
            address: url.host,
            client: self,
            protocol: url.protocol,
            request,
        })
    }

    /// Sends a raw request to the given address.
    pub fn request(
        &self,
        address: impl ToSocketAddrs,
        request: Request,
    ) -> Result<Response, Box<dyn Error>> {
        let mut stream = TcpStream::connect(address)?;
        let request_bytes: Vec<u8> = request.into();
        stream.write_all(&request_bytes)?;

        let response = Response::from_stream(&mut stream)?;

        Ok(response)
    }

    /// Sends a raw request to the given address using TLS.
    #[cfg(not(feature = "tls"))]
    pub fn request_tls(
        &mut self,
        _: impl ToSocketAddrs,
        _: Request,
    ) -> Result<Response, Box<dyn Error>> {
        Err("TLS feature is not enabled".into())
    }

    /// Sends a raw request to the given address using TLS.
    #[cfg(feature = "tls")]
    pub fn request_tls(
        &mut self,
        address: impl ToSocketAddrs,
        request: Request,
    ) -> Result<Response, Box<dyn Error>> {
        if self.tls_config.is_none() {
            let mut roots = RootCertStore::empty();
            for cert in load_native_certs()? {
                roots.add(&Certificate(cert.0))?;
            }

            let conf = ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(roots)
                .with_no_client_auth();

            self.tls_config = Some(Arc::new(conf))
        }

        let conn = ClientConnection::new(
            self.tls_config.as_ref().unwrap().clone(),
            request
                .headers
                .get(&RequestHeader::Host)
                .unwrap()
                .as_str()
                .try_into()
                .unwrap(),
        )?;
        let sock = TcpStream::connect(address)?;
        let mut tls = StreamOwned::new(conn, sock);

        let request_bytes: Vec<u8> = request.into();
        tls.write_all(&request_bytes)?;

        let response = Response::from_stream(&mut tls)?;

        Ok(response)
    }

    /// Parses a URL into a URL struct.
    fn parse_url(url: impl AsRef<str>) -> Option<ParsedUrl> {
        let url = url.as_ref();

        if let Some(stripped) = url.strip_prefix("http://") {
            let protocol = Protocol::Http;
            let (host, path) = stripped.split_once('/').unwrap_or((stripped, ""));

            let mut headers = RequestHeaderMap::new();
            headers.insert(RequestHeader::Host, host.to_string());

            let host = format!("{}:80", host);
            let host = host.to_socket_addrs().ok()?.next()?;

            let (path, query) = path.split_once('?').unwrap_or((path, ""));

            Some(ParsedUrl {
                protocol,
                host,
                host_headers: headers,
                path: format!("/{}", path),
                query: query.to_string(),
            })
        } else if let Some(stripped) = url.strip_prefix("https://") {
            let protocol = Protocol::Https;
            let (host, path) = stripped.split_once('/').unwrap_or((stripped, ""));

            let mut headers = RequestHeaderMap::new();
            headers.insert(RequestHeader::Host, host.to_string());

            let host = format!("{}:443", host);
            let host = host.to_socket_addrs().ok()?.next()?;

            let (path, query) = path.split_once('?').unwrap_or((path, ""));

            Some(ParsedUrl {
                protocol,
                host,
                host_headers: headers,
                path: format!("/{}", path),
                query: query.to_string(),
            })
        } else {
            None
        }
    }
}

/// Represents a request to be sent.
pub struct ClientRequest<'a> {
    client: &'a mut Client,
    protocol: Protocol,
    address: SocketAddr,
    request: Request,
}

impl<'a> ClientRequest<'a> {
    /// Adds a header to the request.
    pub fn with_header(mut self, header: RequestHeader, value: impl AsRef<str>) -> Self {
        self.request
            .headers
            .insert(header, value.as_ref().to_string());
        self
    }

    /// Sends the request.
    pub fn send(self) -> Result<Response, Box<dyn Error>> {
        match self.protocol {
            Protocol::Http => self.client.request(self.address, self.request),
            Protocol::Https => self.client.request_tls(self.address, self.request),
        }
    }
}

struct ParsedUrl {
    protocol: Protocol,
    host: SocketAddr,
    host_headers: RequestHeaderMap,
    path: String,
    query: String,
}

enum Protocol {
    Http,
    Https,
}
