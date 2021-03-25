//! A reverse proxy server

use std::{io, net};

use actix::prelude::*;
use futures::future;
use futures::*;
use hyper::{self, client, Body, Method, Uri};
use hyper_tls;
use log::*;
use tokio;

use crate::ps::agent::messages::{Response, ServerStartup};
use crate::ps::agent::types::{HostName, Server, ServiceId, WithProps};
use crate::ps::agent::{server, Future as AgentFuture};
use crate::ps::util::futures::*;

#[cfg(debug_assertions)]
use crate::ps::util::http::inspect_request;
use crate::ps::util::{actor as a, http as h};

#[cfg(all(unix, target_os = "macos"))]
const ARCHITECTURE: &str = "mac";
#[cfg(all(unix, not(target_os = "macos")))]
const ARCHITECTURE: &str = "unix";
#[cfg(windows)]
const ARCHITECTURE: &str = "windows";

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const X_PS_API_LOCATION: &str = "X-Ps-Api-Location";

/// An agent server that acts as a reverse proxy
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ReverseProxyServer;

#[derive(Clone)]
pub struct Props {
    pub hostname: HostName,
    pub remote_port: u16,
}

impl Actor for ReverseProxyServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} actor", self.id());
    }
}

impl Supervised for ReverseProxyServer {}

impl SystemService for ReverseProxyServer {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} system service", self.id());
    }
}

impl WithProps for ReverseProxyServer {
    type Props = Props;
}

// It is also possible to return a Future here as well (see `ServiceFuture`):
impl Handler<ServerStartup> for ReverseProxyServer {
    type Result = ();

    fn handle(&mut self, msg: ServerStartup, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        Arbiter::spawn(self.listen(msg.addr).map_err(move |e| {
            e.render_with_context(id);
            a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
        }))
    }
}

impl Server for ReverseProxyServer {
    fn id(&self) -> ServiceId {
        ServiceId("ReverseProxy")
    }
}

impl ReverseProxyServer {
    fn listen(&self, sockaddr: net::SocketAddr) -> AgentFuture<()> {
        let id = self.id();
        let props: Props = self
            .get_props()
            .unwrap_or_else(|| panic!("{:?}: missing props", id));

        let hostname = props.hostname;
        let port = props.remote_port;

        match tokio::net::TcpListener::bind(&sockaddr) {
            Ok(listener) => {
                let protocol: hyper::server::conn::Http = hyper::server::conn::Http::new();
                listener
                    .incoming()
                    .for_each(move |sock| {
                        let https_connector = hyper_tls::HttpsConnector::new(4)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        let client = hyper::Client::builder().build(https_connector);
                        let service = ReverseProxyService::new(client, hostname.clone(), port);

                        let response = protocol.serve_connection(sock, service).map_err(move |e| {
                            error!("{:?} :: \n{}", id, e.to_string());
                        });

                        Arbiter::spawn(response);

                        Ok(())
                    })
                    .map_err(Into::into)
                    .into_trait()
            }
            Err(e) => future::err(e.into()).into_trait(),
        }
    }
}

/// Hyper: Reverse proxy service
///
/// Using `hyper::Client`, this `hyper::server::Service` implementation
/// makes an outgoing HTTP request using the method, path, headers, query
/// parameters, and body used for the original incoming request to the local
/// proxy server, returning the remote response.
struct ReverseProxyService {
    // The Hyper client responsible for making outgoing requests
    client: hyper::Client<hyper_tls::HttpsConnector<client::HttpConnector>>,
    // The remote host to proxy the request to:
    remote_host: HostName,
    // The remote port to proxy the request to:
    remote_port: u16,
    // User agent that is added to all requests
    user_agent: String,
}

impl ReverseProxyService {
    // Create a ReverseProxyService instance
    pub fn new(
        client: hyper::Client<hyper_tls::HttpsConnector<client::HttpConnector>>,
        remote_host: HostName,
        remote_port: u16,
    ) -> Self {
        ReverseProxyService {
            client,
            remote_host,
            remote_port,
            user_agent: format!("agent/{}/{}", ARCHITECTURE, AGENT_VERSION),
        }
    }
}

impl hyper::service::Service for ReverseProxyService {
    type ReqBody = hyper::Body;
    type ResBody = hyper::Body;
    type Error = hyper::Error;
    type Future =
        Box<dyn Future<Item = hyper::Response<Self::ResBody>, Error = Self::Error> + Send>;

    // Here's a nice feature of Hyper: since the response body provided by the
    // outgoing Client request implements the `futures::Stream` trait, the
    // body can be returned directly by the server as the `Response` body:
    //
    // See https://users.rust-lang.org/t/how-can-i-forward-the-stream-of-a-hyper-client-response-to-a-hyper-sever-response-proxy/11511/2
    fn call(&mut self, req: hyper::Request<Self::ReqBody>) -> Self::Future {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/health") => {
                let mut response = hyper::Response::default();
                *response.status_mut() = hyper::StatusCode::OK;
                future::ok(response).into_trait()
            }
            _ => {
                #[cfg(debug_assertions)]
                inspect_request(&req);

                let remote_host: String = match req.headers().get(X_PS_API_LOCATION) {
                    Some(api_location) => {
                        let redirect_api_loc = match api_location.to_str() {
                            Ok(location) => location,
                            Err(_) => {
                                return future::ok(h::fail_with_message(format!(
                                    "Bad API location: {:?}",
                                    api_location
                                )))
                                .into_trait();
                            }
                        };
                        match HostName::parse(redirect_api_loc) {
                            Ok(uri) => uri.into(),
                            Err(_) => {
                                return future::ok(h::fail_with_message(format!(
                                    "Bad API location: {:?}",
                                    redirect_api_loc
                                )))
                                .into_trait();
                            }
                        }
                    }
                    _ => self.remote_host.clone().into(),
                };

                // The outgoing request should have the same path and query parameters
                // as the original request:
                let remote_uri_str = format!("{}:{}{}", remote_host, self.remote_port, &req.uri());

                let remote_uri: Uri = match remote_uri_str.parse() {
                    Ok(uri) => uri,
                    Err(_) => {
                        return future::ok(h::fail_with_message(format!(
                            "Bad URI: {}",
                            remote_uri_str
                        )))
                        .into_trait();
                    }
                };

                // Make a new outgoing request using the same headers, body, etc
                // as the original:
                let mut outgoing_req: hyper::Request<Body> = hyper::Request::default();
                {
                    let outgoing_headers = outgoing_req.headers_mut();
                    let incoming_headers = req.headers();

                    *outgoing_headers = incoming_headers.clone();

                    // We need to explicitly remove the `Host: 127.0.0.1:{local-port}`,
                    // as it does not correspond to the remote server the request is
                    // being made to (this is required).
                    outgoing_headers.remove(hyper::header::HOST);
                    outgoing_headers.remove(hyper::header::CONNECTION);
                    outgoing_headers.remove(X_PS_API_LOCATION);

                    // Add a user agent header for tracking
                    outgoing_headers.insert(
                        hyper::header::USER_AGENT,
                        hyper::header::HeaderValue::from_str(&self.user_agent.clone()).unwrap(),
                    );
                }

                let uri_as_string = remote_uri.to_string();

                // Send an update to the status server:
                a::send_unconditionally::<server::StatusServer, _>(
                    Response::incoming_proxy_request(uri_as_string),
                );

                *outgoing_req.method_mut() = req.method().clone();
                *outgoing_req.uri_mut() = remote_uri;
                *outgoing_req.body_mut() = req.into_body();

                #[cfg(debug_assertions)]
                inspect_request(&outgoing_req);

                to_future_trait(self.client.request(outgoing_req))
            }
        }
    }
}
