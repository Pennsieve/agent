//! Status reporting endpoint
use std::cell::RefCell;
use std::collections::HashSet;
use std::time::Duration;

use ::actix::prelude::*;
use actix_net::server as s;
use actix_web::server::HttpServer;
use actix_web::*;
use log::*;
use serde_json::{self, Value as JSON};

use crate::ps::agent::messages::{self, *};
use crate::ps::agent::{server, upload};
use crate::ps::util::actor as a;

////////////////////////////////////////////////////////////////////////////////
// Messages
////////////////////////////////////////////////////////////////////////////////

/// Message: send JSON payload messages to the websocket frontend.
#[derive(Clone, Debug)]
struct SendPayload(pub JSON);

impl Message for SendPayload {
    type Result = server::Result<()>;
}

/// Message: register a websocket server with the status server.
#[derive(Message)]
struct RegisterWebSocket {
    pub addr: Addr<WebSocketServer>,
}

impl RegisterWebSocket {
    fn new(addr: Addr<WebSocketServer>) -> Self {
        Self { addr }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Status server shared state
////////////////////////////////////////////////////////////////////////////////

/// Thread local actor state:
thread_local! {
    static CLIENTS: RefCell<HashSet<Addr<WebSocketServer>>> = RefCell::new(HashSet::new());
}

////////////////////////////////////////////////////////////////////////////////
// Websocket shared state
////////////////////////////////////////////////////////////////////////////////

pub struct WebsocketSharedState {
    /// The actix-web state shared amongst all web socket server instances.
    status_addr: Addr<StatusServer>,
}

impl WebsocketSharedState {
    /// Create a new shared websocket state.
    fn new(status_addr: Addr<StatusServer>) -> Self {
        Self { status_addr }
    }

    /// Get the address of the status server.
    fn status_addr(&self) -> &Addr<StatusServer> {
        &self.status_addr
    }
}

// Like `Props` instances for the various servers and workers, the thread-local
// state for this module contains the current, active web socket server
// instances. This is needed due to the restriction of `Default` being
// required on the `SystemService` trait implemented for `StatusServer`. The
// trait is required so that messages can be sent directly between actors
// by way of `System::current().registry().get::<T>().do_send(...)`, where type
// `T` must implement `Default`, `Supervised`, and `SystemService`.
#[derive(Default)]
pub struct StatusServer;

impl StatusServer {
    /// Create a new status server instance.
    pub fn new() -> Self {
        Self
    }

    /// Register a web socket address with the status server, so that
    /// messages can be sent to it later.
    fn register_websocket_addr(&self, ws_addr: Addr<WebSocketServer>) {
        CLIENTS.with(|ws| {
            ws.borrow_mut().insert(ws_addr);
        })
    }

    /// Send a status message to the websocket server.
    fn send_to_websocket(&mut self, payload: JSON) -> server::Result<()> {
        CLIENTS.with(|ws| {
            for addr in ws.borrow().iter() {
                if addr.connected() {
                    addr.do_send(SendPayload(payload.clone()));
                }
            }
        });
        Ok(())
    }

    // Removes disconnected `Addr` instances from the `CLIENTS` thread-local
    // variable:
    fn cleanup_dead_addresses() {
        CLIENTS.with(|ws| ws.borrow_mut().retain(|ref addr| addr.connected()));
    }
}

impl Actor for StatusServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("started status actor");

        // Run a cleanup job every 2 seconds to reap disconnected addresses:
        ctx.run_interval(Duration::from_millis(2_000), |_, _| {
            Self::cleanup_dead_addresses()
        });
    }
}

impl Supervised for StatusServer {}

impl SystemService for StatusServer {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started status system service");
    }
}

impl Handler<RegisterWebSocket> for StatusServer {
    type Result = ();

    fn handle(&mut self, msg: RegisterWebSocket, _ctx: &mut Self::Context) -> Self::Result {
        self.register_websocket_addr(msg.addr);
    }
}

impl Handler<SystemShutdown> for StatusServer {
    type Result = ();

    fn handle(&mut self, msg: SystemShutdown, _ctx: &mut Self::Context) -> Self::Result {
        info!("received message::SystemShutdown = {:?}", msg);
        info!("*** Calling System::current().stop() ***");
        System::current().stop();
    }
}

impl Handler<StartStatusServer> for StatusServer {
    type Result = server::Result<Addr<s::Server>>;

    fn handle(&mut self, msg: StartStatusServer, ctx: &mut Self::Context) -> Self::Result {
        let port = msg.port;
        let self_addr: Addr<StatusServer> = ctx.address();

        info!("Server status websocket running on 0.0.0.0:{}", port);

        let http_server_addr: Addr<_> = HttpServer::new(move || {
            let self_addr = self_addr.clone();
            App::with_state(WebsocketSharedState::new(self_addr)).resource("/", move |r| {
                r.route().f(move |req| ws::start(req, WebSocketServer))
            })
        })
        .bind(format!("0.0.0.0:{}", port))?
        .start();

        Ok(http_server_addr)
    }
}

impl Handler<messages::Response> for StatusServer {
    type Result = server::Result<()>;

    fn handle(&mut self, msg: messages::Response, _ctx: &mut Self::Context) -> Self::Result {
        info!("status message: received response = {:?}", msg);
        let payload: JSON = serde_json::to_value(&msg)?;
        self.send_to_websocket(payload)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Web socket frontend
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct WebSocketServer;

impl Actor for WebSocketServer {
    type Context = ws::WebsocketContext<Self, WebsocketSharedState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("status server: listening on websocket");
        // Register the websocket server with the status server:
        let self_addr: Addr<WebSocketServer> = ctx.address();
        let state: &WebsocketSharedState = ctx.state();
        state
            .status_addr()
            .do_send(RegisterWebSocket::new(self_addr));
    }
}

impl Handler<SendPayload> for WebSocketServer {
    type Result = server::Result<()>;

    fn handle(&mut self, msg: SendPayload, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&msg.0)?);
        Ok(())
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WebSocketServer {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Text(text) => {
                // Attempt to decode the text as a `Request` instance:
                match serde_json::from_str::<messages::Request>(&text) {
                    Ok(request) => {
                        info!("websocket: request OK = {:#?}", request);
                        match request {
                            messages::Request::QueueUpload { body: queue_upload } => {
                                a::send_unconditionally::<upload::worker::Uploader, _>(
                                    queue_upload,
                                );
                            }
                        }
                    }
                    Err(_e) => {
                        error!("malformed websocket message = {}", text);
                    }
                }
            }
            ws::Message::Ping(body) => {
                info!("PING");
                ctx.pong(&body)
            }
            _ => (),
        }
    }
}
