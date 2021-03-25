//! Timeseries web-socket proxy

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{cmp, collections, io};

use actix::prelude::*;
use futures::prelude::*;
use futures::sync::oneshot;
use futures::{future as f, stream as st, Future as _Future};
use log::*;
use protobuf::{self, Message};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_hdr_async, connect_async};
use tungstenite;
use tungstenite::handshake::server::{Callback, Request};
#[allow(unused_imports)]
use tungstenite::protocol::Message as WsMessage;
use url;

use pennsieve_macros::try_future;

use crate::ps::agent::database::Database;
use crate::ps::agent::messages::{Response, ServerStartup};
use crate::ps::agent::server;
use crate::ps::agent::types::{HostName, Server, ServiceId, TxStop, WithProps};
use crate::ps::agent::{self, cache, Future};
use crate::ps::proto::timeseries::{AgentTimeSeriesResponse, StateMessage, TimeSeriesMessage};
use crate::ps::util::actor as a;
use crate::ps::util::futures::*;

use super::{Error, ErrorKind, Result};

/// The number of concurrent chunks that can be sent concurrently to the client.
const CONCURRENT_REQUEST_CHUNK_LIMIT: usize = 50;

/// Websocket command state response: READY
const READY: &str = "READY";

/// Websocket command state response: NOT_READY
const NOT_READY: &str = "NOT_READY";

/// Websocket command state response: ERROR
const ERROR: &str = "ERROR";

/// Websocket command state response: DONE
const DONE: &str = "DONE";

#[derive(Debug)]
struct AcceptCallback(oneshot::Sender<String>);

impl Callback for AcceptCallback {
    fn on_request(
        self,
        request: &Request,
    ) -> tungstenite::error::Result<Option<Vec<(String, String)>>> {
        if self.0.send(request.path.clone()).is_err() {
            // ignore
        }
        Ok(None)
    }
}

/// Given a remote host name, remote port and a url path, this functions
/// constructs a URL
fn remote_url(remote_host: HostName, remote_port: u16, path: &str) -> Result<url::Url> {
    let hostname: String = remote_host.into();
    match url::Url::parse(&hostname) {
        Ok(url) => {
            let mut url2 = url.join(path)?;
            if url2.set_port(Some(remote_port)).is_err() {
                Err(ErrorKind::InvalidPort {
                    hostname,
                    port: remote_port,
                }
                .into())
            } else {
                Ok(url2)
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Given a vector of unsigned bytes, attempt to convert the bytes into a
/// typeful representation of a timeseries message.
fn into_timeseries(data: &[u8]) -> Result<TimeSeriesMessage> {
    protobuf::parse_from_bytes(data).map_err(Into::into)
}

/// Generate a state message as a protobuf Vec of bytes
fn state_message_bytes<S, D>(status: S, description: Option<D>) -> Vec<u8>
where
    S: Into<String>,
    D: Into<String>,
{
    let mut state = StateMessage::new();
    state.set_status(status.into());

    if let Some(d) = description {
        state.set_description(d.into())
    }

    let mut response = AgentTimeSeriesResponse::new();
    response.set_state(state);

    response.write_to_bytes().unwrap_or_else(|e| {
        error!("state_message_bytes :: {:?}", e);
        Vec::new()
    })
}

/// Generate a "READY" state status message
fn status_ready() -> WsMessage {
    WsMessage::Binary(state_message_bytes(READY, None as Option<&str>))
}

/// Generate a "NOT_READY" state status message
fn status_not_ready() -> WsMessage {
    WsMessage::Binary(state_message_bytes(NOT_READY, None as Option<&str>))
}

/// Generate an "ERROR" state status message
fn status_error<S: Into<String>>(description: S) -> WsMessage {
    let description = description.into();
    WsMessage::Binary(state_message_bytes(ERROR, Some(description.as_str())))
}

/// Generate a "DONE" state status message
fn status_done() -> WsMessage {
    WsMessage::Binary(state_message_bytes(DONE, None as Option<&str>))
}

// All messages with a matching (source, start-time, end-time) tuple will
// hash to the same bucket and be considered part of the same message group:
type MessageGroupKey = (String, u64, u64);

/// Holds the internal state needed to know when a TimeSeriesStream has finished
/// processing a chunk. This contains fields of `Cell`s and `RefCell`s because
/// this state is changed per request chunk. The request chunk acts as the `Sink`
/// for this stream.
struct TimeSeriesStreamState {
    pending_requests: usize,
    received_parts: collections::HashMap<MessageGroupKey, u64>,
    done: bool,
}

impl TimeSeriesStreamState {
    /// Create a new `TimeSeriesStreamState`. Holds internal state needed to manage
    /// a `TimeSeriesStream`.
    fn new(total_sent_requests: usize) -> Self {
        Self {
            pending_requests: total_sent_requests,
            received_parts: collections::HashMap::new(),
            done: false,
        }
    }

    fn done() -> Self {
        Self {
            pending_requests: 0,
            received_parts: collections::HashMap::new(),
            done: true,
        }
    }

    /// Resets the internal fields for the given sent request count. Resetting
    /// these fields allows the underlying stream to process a new set of messages.
    fn reset(&mut self, total_sent_requests: usize) {
        self.pending_requests = total_sent_requests;
        self.received_parts.clear();
        self.done = false;
    }
}

/// A stream that wraps another stream that produces timeseries messages. When
/// all channel messages are received, this stream will terminate.
struct TimeSeriesStream<S: Stream> {
    inner: S,
    state: Arc<Mutex<TimeSeriesStreamState>>,
}

impl<S: Stream> TimeSeriesStream<S> {
    fn new(inner: S, total_sent_requests: usize) -> TimeSeriesStream<S> {
        Self {
            inner,
            state: Arc::new(Mutex::new(TimeSeriesStreamState::new(total_sent_requests))),
        }
    }

    fn empty(inner: S) -> TimeSeriesStream<S> {
        Self {
            inner,
            state: Arc::new(Mutex::new(TimeSeriesStreamState::done())),
        }
    }

    fn state(&self) -> Arc<Mutex<TimeSeriesStreamState>> {
        Arc::clone(&self.state)
    }

    // This method monitors time series messages emitted from the timeseries
    // server. Since we don't know the number of the messages we're going to
    // receive ahead of time, we need to check each message for two fields:
    // `totalResponses` and `responseSequenceId`. `totalResponses` encodes how
    // many messages are part of the same message group, and `responseSequenceId`
    // specifies the number of an individual message in the sequence.
    fn update_message_counts(&mut self, binary_data: &[u8]) {
        if let Ok(ts) = into_timeseries(binary_data) {
            let mut state = self.state.lock().unwrap();

            // Pull out the source channel, and page start and end times:
            if let Some(segment) = ts.segment.into_option() {
                // Get the total expected responses and the sequence ID of the current message:
                let total_responses = ts.totalResponses;
                let seq_id = ts.responseSequenceId;

                // If the message is a singleton, decrement the pending
                // request count immediately:
                if seq_id == 0 && total_responses == 1 {
                    state.pending_requests = cmp::max(state.pending_requests - 1, 0);
                    return;
                }

                // Construct a key used to identify the message. All messages
                // identified by the messages the same (source, start, end)
                // tuple will hash to the same bucket.
                let key = (segment.source, segment.pageStart, segment.pageEnd);

                // Track the total number of pending requests that still need to be made:
                let mut pending_requests = state.pending_requests;
                let mut done = false;

                if let Some(group_count) = state.received_parts.get_mut(&key) {
                    // Decrement the number of messages in the message group:
                    *group_count = cmp::max(*group_count - 1, 0);

                    // If the group count becomes 0, decrement the total
                    // number of pending requests:
                    if *group_count == 0 {
                        pending_requests = cmp::max(pending_requests - 1, 0);
                    }

                    done = true;
                }

                state.pending_requests = pending_requests;

                if done {
                    return;
                }

                // Decrement the number of expected parts for a message group
                state.received_parts.insert(key, total_responses - 1);
            } else {
                error!("Received timeseries message didn't contain a Segment. The agent does not support Event data.");
            }
        } else {
            error!("Received message that wasn't parsable into a TimeSeriesMessage.");
        }
    }

    fn all_messages_received(&self) -> bool {
        self.state.lock().unwrap().pending_requests == 0
    }
}

impl<S> Stream for TimeSeriesStream<S>
where
    S: Stream<Item = WsMessage, Error = tungstenite::error::Error>,
{
    type Item = S::Item;
    type Error = S::Error;

    /// Polls for timeseries messages. When all messages are received, the
    /// stream will terminate.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(Some(msg))) => {
                if let WsMessage::Binary(ref data) = msg {
                    // Keep polling for arrived timeseries messages while the following
                    // conditions hold:
                    //
                    // (1) The number previously seen channel IDs matches the number of
                    //     channels specified in the request
                    // (2) The number of remaining messages per channel ID is zero
                    self.update_message_counts(data);

                    if self.all_messages_received() {
                        self.state.lock().unwrap().done = true;
                    }
                }
                Ok(Async::Ready(Some(msg)))
            }
            r @ Ok(Async::NotReady) => {
                if self.state.lock().unwrap().done {
                    return Ok(Async::Ready(None));
                }
                r
            }
            other => {
                if self.state.lock().unwrap().done {
                    return Ok(Async::Ready(None));
                }
                other
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// A request for a time series message to be sent to the Pennsieve streaming
/// API:
struct ApiRequest {
    session: String,
    channels: Vec<String>,
    start_time: u64,
    end_time: u64,
    package_id: String,
    pixel_width: i32,
    query_limit: Option<i64>,
}

impl ApiRequest {
    fn new(
        session: String,
        package_id: String,
        channels: Vec<String>,
        start_time: u64,
        end_time: u64,
    ) -> Self {
        ApiRequest {
            session,
            channels,
            package_id,
            start_time,
            end_time,
            pixel_width: -1,
            query_limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRequest {
    id: String,
    rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// A request for a time series message to be sent to the Pennsieve agent:
///
/// {
///   "session": "dfb723f6-f8ad-4387-b589-7c91f838a5fa",
///   "packageId": "N:package:10e6b2d8-e0ce-47e3-ab60-e18db7c91a38",
///   "channels": [
///     {
///       "id": "N:channel:033df6d6-2f0c-4a2b-888d-429131dfd213",
///       "rate": 200.0
///     },
///     {
///       "id": "N:channel:29c3ad8d-f9a3-4c3e-bfbe-91b38399c8c0",
///       "rate": 500.0
///     }
///   ],
///   "startTime": 946684885000000,
///   "endTime": 946684890000000,
///   "chunkSize": 20000,
///   "useCache": true
/// }
pub struct AgentRequest {
    session: String,
    package_id: String,
    channels: Vec<ChannelRequest>,
    start_time: u64,
    end_time: u64,
    chunk_size: u64,
    use_cache: Option<bool>,
}

// Convert an `cache::PageRequest` to an `APIRequest`

fn into_api_request(session: &str, package_id: &str, page: &cache::PageRequest) -> ApiRequest {
    let session = session.to_string();
    let chs_id = vec![page.channel_id().to_string()];
    let start = page.start();
    let end = page.end();
    ApiRequest::new(session, package_id.to_string(), chs_id, start, end)
}

// Convert an `AgentRequest` to an `APIRequest`
impl From<AgentRequest> for ApiRequest {
    fn from(req: AgentRequest) -> Self {
        ApiRequest {
            session: req.session.clone(),
            package_id: req.package_id.clone(),
            channels: req.channels.iter().map(|c| c.id.clone()).collect(),
            start_time: req.start_time,
            end_time: req.end_time,
            pixel_width: -1,
            query_limit: None,
        }
    }
}

impl From<ChannelRequest> for cache::Channel {
    fn from(channel: ChannelRequest) -> Self {
        cache::Channel::new(channel.id, channel.rate)
    }
}

impl From<AgentRequest> for cache::Request {
    fn from(req: AgentRequest) -> Self {
        cache::Request::new(
            req.package_id,                                       // package_id
            req.channels.into_iter().map(|c| c.into()).collect(), // channels
            req.start_time,                                       // start
            req.end_time,                                         // end
            req.chunk_size as u32,                                // chunk_size
            req.use_cache.unwrap_or(false),                       // use_cache
        )
    }
}
// ============================================================================

/// Commands sent to the the time series server from a client (Python, R, etc.)
///
/// # "new"
///
///   Create a new request for an iterator over chunks of timeseries data
///
/// ## Example
///
///   {
///     "command": "new",
///     "session": "dfb723f6-f8ad-4387-b589-7c91f838a5fa",
///     "packageId": "N:package:10e6b2d8-e0ce-47e3-ab60-e18db7c91a38",
///     "channels": [
///       {
///         "id": "N:channel:033df6d6-2f0c-4a2b-888d-429131dfd213",
///         "rate": 200.0
///       },
///       {
///         "id": "N:channel:29c3ad8d-f9a3-4c3e-bfbe-91b38399c8c0",
///         "rate": 500.0
///       }
///     ],
///     "startTime": 946684885000000,
///     "endTime": 946684890000000,
///     "chunkSize": 20000,
///     "useCache": true
///    }
///
/// # "next"
///
///   Advances the iterator to the next available chunk
///
/// ## Example
///
///    { "command": "next" }
///
/// # "close"
///
///   Closes the iterator
///
/// ## Example
///
///   { "command": "close" }
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "command")]
enum Command {
    New(AgentRequest),
    Next,
    Close,
}

/// A type that represents the current state of the server from the view of
/// an individual client. This state holds the chunk reponse iterator (along
/// with its own state) specific to a given client.
struct LoopState<S: Send + Sink> {
    sink: S,
    config: cache::Config,
    db: Database,
    tx_kill: Option<TxStop>,
    chunk_iter: Option<cache::ChunkResponseIterator>,
}

impl<S: Send + Sink> LoopState<S> {
    pub fn new(sink: S, config: cache::Config, db: Database, tx_kill: TxStop) -> Self {
        Self {
            sink,
            config,
            db,
            tx_kill: Some(tx_kill),
            chunk_iter: None,
        }
    }

    #[allow(unknown_lints, clippy::wrong_self_convention)]
    pub fn to_iterator(state: LoopState<S>, response: cache::Response) -> Self {
        Self {
            sink: state.sink,
            config: state.config,
            db: state.db.clone(),
            tx_kill: state.tx_kill,
            chunk_iter: Some(response.owned_chunk_response_iter(state.db)),
        }
    }

    pub fn get_config(&self) -> &cache::Config {
        &self.config
    }

    pub fn get_db(&self) -> &Database {
        &self.db
    }

    pub fn kill(self) -> Result<Self> {
        if let Some(kill) = self.tx_kill {
            kill.send(())
                .map_err(|_| Into::<Error>::into(ErrorKind::ShutdownError))?;
        }
        Ok(Self {
            sink: self.sink,
            config: self.config,
            db: self.db,
            tx_kill: None,
            chunk_iter: self.chunk_iter,
        })
    }

    /// Split the loop state `s` into a tuple (`s'`, iterator), with `s'`
    /// taking ownership of the internal state of `s`.
    pub fn split(self) -> (Self, Option<cache::ChunkResponseIterator>) {
        let iter = self.chunk_iter;
        let prime = Self {
            sink: self.sink,
            config: self.config,
            db: self.db,
            tx_kill: self.tx_kill,
            chunk_iter: None,
        };
        (prime, iter)
    }

    /// The inverse of `split`: join loop state `s` with a chunk iterator
    /// to produce a new state `s'` that assumes ownership of `s` internal
    /// state.
    pub fn join(self, chunk_iter: Option<cache::ChunkResponseIterator>) -> Self {
        Self {
            sink: self.sink,
            config: self.config,
            db: self.db,
            tx_kill: self.tx_kill,
            chunk_iter,
        }
    }

    pub fn send_message(&mut self, payload: S::SinkItem) -> Option<S::SinkError> {
        match self.sink.start_send(payload) {
            Ok(_) => None,
            Err(e) => Some(e),
        }
    }

    pub fn close(&mut self) -> Option<S::SinkError> {
        match self.sink.close() {
            Ok(_) => None,
            Err(e) => Some(e),
        }
    }
}

// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct TimeSeriesServer;

#[derive(Clone)]
pub struct Props {
    pub hostname: HostName,
    pub port: u16,
    pub config: cache::Config,
    pub db: Database,
}

impl Actor for TimeSeriesServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} actor", self.id());
    }
}

impl Supervised for TimeSeriesServer {}

impl SystemService for TimeSeriesServer {
    fn service_started(&mut self, _ctx: &mut Self::Context) {
        info!("started {:?} system service", self.id());
    }
}

impl WithProps for TimeSeriesServer {
    type Props = Props;
}

// It is also possible to return a Future here as well (see `ServiceFuture`):
impl Handler<ServerStartup> for TimeSeriesServer {
    type Result = ();

    fn handle(&mut self, msg: ServerStartup, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id();
        Arbiter::spawn(self.listen(msg.addr).map_err(move |e| {
            e.render_with_context(id);
            a::send_unconditionally::<server::StatusServer, _>(Response::error(e));
        }))
    }
}

impl Server for TimeSeriesServer {
    fn id(&self) -> ServiceId {
        ServiceId("TimeSeries")
    }
}

impl TimeSeriesServer {
    fn listen(&self, sockaddr: SocketAddr) -> Future<()> {
        let id = self.id();
        let props: Props = self
            .get_props()
            .unwrap_or_else(|| panic!("{:?}: missing props", id));

        let listener = try_future!(TcpListener::bind(&sockaddr));

        let hostname = props.hostname;
        let port: u16 = props.port;
        let config = props.config;
        let db = props.db;
        let page_creator = cache::PageCreator::new();

        listener
            .incoming()
            .for_each(move |sock| {
                let hostname = hostname.clone();
                let config = config.clone();
                let db = db.clone();
                let page_creator = page_creator.clone();

                // Build a channel so the path and query parameters of
                // the incoming websocket url can be sent from `AcceptCallback.onRequest()` method:
                let (tx_req_path, rx_req_path) = oneshot::channel::<String>();

                // A trigger to kill the server instance when a
                // signal is sent upon the "close" command being received:
                let (tx_kill, rx_kill) = oneshot::channel::<()>();

                // Start the server:
                accept_hdr_async(sock, AcceptCallback(tx_req_path))
                    .join3(
                        rx_req_path.map_err(|e| {
                            // Wait to receive a notification containing the URL of the request:
                            tungstenite::error::Error::Io(io::Error::new(io::ErrorKind::Other, e))
                        }),
                        Ok(tx_kill),
                    )
                    // Ok, we're listening for new connections by this point:
                    .and_then(move |(ws_client_stream, request_path, tx_kill)| {

                        // Split the websocket stream into a (sink, source) pair:
                        let (client_sink, client_stream) = ws_client_stream.split();

                        // Use a channel to send requests to the agent from from the user
                        // (Python, Matlab, R, etc.) by way of the source stream to the sink:
                        let (tx_command, rx_command) = futures::sync::mpsc::channel::<Result<Command>>(16);

                        // For each message in the incoming stream from the user, attempt
                        // to parse the raw text into a typeful `Command` representation
                        // that the receiver will be able to act upon:
                        let read_client_commands = tx_command
                            .sink_map_err(|e| Into::<agent::Error>::into(Into::<Error>::into(e)))
                            .send_all(
                                client_stream
                                    .map_err(|e| Into::<agent::Error>::into(Into::<Error>::into(e)))
                                    .map(|msg: WsMessage| {
                                        match msg {
                                            WsMessage::Text(ref text) => serde_json::from_str::<Command>(text)
                                                .map_err(Into::<Error>::into),
                                            WsMessage::Binary(_) => Err(Error::invalid_message_type("binary")),
                                            WsMessage::Ping(_) => Err(Error::invalid_message_type("ping")),
                                            WsMessage::Pong(_) => Err(Error::invalid_message_type("pong"))
                                        }
                                    }));

                        // Set up the initial state of the client command loop:
                        let loop_state: LoopState<_> = LoopState::new(client_sink, config, db, tx_kill);

                        let dispatch_client_commands = rx_command
                            .map_err(|_| ErrorKind::ShutdownError.into())
                            .fold(loop_state, move |mut state: LoopState<_>, command: Result<Command>| -> agent::Future<LoopState<_>> {
                                match command {
                                    // Bad command: respond to the client with the error. Break out
                                    // of the command loop:
                                    Err(e) => {
                                        state.send_message(status_error(e.to_string()));
                                        f::err(e.into()).into_trait()
                                    },

                                    Ok(Command::New(query_request)) => {
                                        // Transform the query request to the agent to a request format
                                        // suitable to send to the Pennsieve streaming API:
                                        let api_request: ApiRequest = query_request.clone().into();

                                        // Transform the request into a caching request:
                                        let cache_request: cache::Request = query_request.into();

                                        // Generate the URL of the streaming server based on
                                        // the query parameters the client sent to the agent:
                                        let streaming_api_url = match remote_url(hostname.clone(), port, &request_path) {
                                            Ok(url) => url,
                                            Err(e) => {
                                                error!("couldn't construct Pennsieve streaming API URL! ~ {:?}", e);
                                                state.send_message(status_error("bad url"));
                                                return f::err(e.into()).into_trait()
                                            }
                                        };

                                        let page_creator = page_creator.clone();

                                        // Create a channel so that received messages can be
                                        // proxied to the Pennsieve streaming timeseries server:
                                        let (tx_streaming_server, rx_streaming_server) = futures::sync::mpsc::channel::<WsMessage>(16);

                                        // Allow access to the response generated from the
                                        // cache request in both this Future task and those
                                        // spawned to handle the subsequent responses returned
                                        // from the streaming server:
                                        let response = Arc::new(Mutex::new(cache_request.get_response(state.get_config())));

                                        let database = Arc::new(state.get_db().clone());

                                        // For each message received through `rx_streaming_server`,
                                        // cache the time series message data to disk,
                                        // while running the code in a separate event loop task:
                                        let cache_messages = rx_streaming_server
                                            .map_err(|_| Error::io_error("error streaming timeseries message"))
                                            .fold(Arc::clone(&response), move |res, msg: WsMessage| -> Result<Arc<Mutex<cache::Response>>>  {
                                                if let WsMessage::Binary(data) = msg {
                                                    match into_timeseries(&data) {
                                                        Ok(ts) => {
                                                            if let Some(segment) = ts.segment.into_option() {
                                                                if let Err(e) = res.lock().unwrap().cache_response(&page_creator, &segment) {
                                                                    Err(e.into())
                                                                } else {
                                                                    Ok(Arc::clone(&res))
                                                                }
                                                            } else {
                                                                Err(ErrorKind::EmptyMessage.into())
                                                            }
                                                       },
                                                        Err(e) => Err(e)
                                                    }
                                                } else {
                                                    Err(Error::invalid_message_type("non-binary"))
                                                }
                                            })
                                            .and_then(|_| {
                                                info!("all messages received");
                                                Ok(())
                                            });

                                        let streaming_api_url_copy = streaming_api_url.clone();

                                        // Connect to the Pennsieve timseries streaming server:
                                        to_future_trait(connect_async(streaming_api_url)
                                                        .map_err(Into::<Error>::into)
                                            .then(move |conn_result| {
                                                match conn_result {
                                                    Err(e) => {
                                                        state.send_message(status_error(format!("ps:timeseries:loop:ws-connect ~ couldn't connect to Pennsieve streaming timeseries server {}", streaming_api_url_copy)));
                                                        f::err(Into::<Error>::into(e)).into_trait()
                                                    },
                                                    Ok((ts_ws_stream, _headers)) => {

                                                        // Split the web socket stream from the Pennsieve timeseries
                                                        // server into a (sink, source) pair:
                                                        let (mut ts_sink, ts_stream) = ts_ws_stream.split();

                                                        // Iterate over all uncached pages, making a request to the
                                                        // streaming web server with the adjusted start/end times
                                                        // for page and send the results to the message channel
                                                        // receiver above:
                                                        let requests_for_caching =
                                                            match response
                                                                .lock()
                                                                .unwrap()
                                                                .uncached_page_requests(&Arc::clone(&database))
                                                            {
                                                                Ok(requests) => requests,
                                                                Err(e) => {
                                                                    state.send_message(status_error(format!("ps:timeseries:loop:ws-connect:uncached-page-iterator ~ {:?}", e)));
                                                                    return f::err(e.into()).into_trait()
                                                                }
                                                            };

                                                        let requests_for_caching = requests_for_caching
                                                            .map(move |page_request: cache::PageRequest| into_api_request(&api_request.session, &api_request.package_id, &page_request))
                                                            .map(move |api_request: ApiRequest| {
                                                                match serde_json::to_string(&api_request) {
                                                                    Ok(json) => {
                                                                        let json_copy = json.clone();
                                                                        match ts_sink.start_send(WsMessage::Text(json)) {
                                                                            Ok(_) => Ok(json_copy),
                                                                            Err(e) => Err(e.into())
                                                                        }
                                                                    },
                                                                    Err(e) => Err(e.into())
                                                                }
                                                            });

                                                        // Wrap a message stream from the Pennsieve server
                                                        // with a custom stream implementation that knows when all
                                                        // messages have been received. Once all messages
                                                        // have been received, the stream will stop polling.
                                                        //
                                                        // If no requests for timeseries data are
                                                        // needed, we can just create an empty stream
                                                        // and resolve immediately:
                                                        let total_requests = requests_for_caching.len();
                                                        let ts_stream = if total_requests > 0 {
                                                            TimeSeriesStream::new(to_stream_trait(ts_stream), 0)
                                                        } else {
                                                            TimeSeriesStream::empty(to_stream_trait(st::empty::<WsMessage, tungstenite::Error>()))
                                                        };

                                                        let stream_state = ts_stream.state();

                                                        // https://stackoverflow.com/questions/43247212/join-futures-with-limited-concurrency
                                                        let send_page_requests = st::iter_ok::<_, Error>(requests_for_caching)
                                                            .chunks(CONCURRENT_REQUEST_CHUNK_LIMIT)
                                                            .fold((0, ts_stream), move |(count, ts_stream), reqs| {

                                                                debug!("Completed {} out of {} requests", count, total_requests);

                                                                let count = count + reqs.len();
                                                                let tx_streaming_server = tx_streaming_server.clone();
                                                                stream_state.lock().unwrap().reset(reqs.len());

                                                                f::join_all(reqs)
                                                                    .and_then(move |_| {
                                                                        tx_streaming_server
                                                                            .sink_map_err(Into::<Error>::into)
                                                                            .send_all(ts_stream)
                                                                            .map(move |(_, stream)| (count, stream))
                                                                    })
                                                            })
                                                            .into_trait();

                                                        // When all sending + receiving tasks are done, we
                                                        // can proceed:
                                                        to_future_trait(cache_messages.join3(send_page_requests, Ok(response))
                                                            .then(move |results| {
                                                                match results {
                                                                    Ok((_, _, response)) => {
                                                                        // By this point, all other pointers
                                                                        // referencing `response` should have gone out
                                                                        // of scope. Since the strong pointer count is
                                                                        // 1, we can unwrap `Arc<cache::Response>` to
                                                                        // its inner `cache::Response` value:
                                                                        if let Ok(response_inner) = Arc::try_unwrap(response) {
                                                                            let inner = match response_inner.into_inner() {
                                                                                Ok(inner) => inner,
                                                                                Err(e) => {
                                                                                    state.send_message(status_error(format!("ps:timeseries:server:response:* ~ {}", e.to_string())));
                                                                                    return Err(Into::<Error>::into(e))
                                                                                }
                                                                            };
                                                                            if let Err(e) = inner.record_page_requests(state.get_db()) {
                                                                                state.send_message(status_error(format!("ps:timeseries:server:record-page-requests ~ {}", e.to_string())));
                                                                                return Err(Into::<Error>::into(e))
                                                                            }
                                                                            info!("sending message <READY>");
                                                                            {
                                                                                state.send_message(status_ready());
                                                                            }
                                                                            Ok(LoopState::to_iterator(state, inner))
                                                                        } else {
                                                                            // Send NOT_READY (realistically, this state shouldn't be reached)
                                                                            state.send_message(status_not_ready());
                                                                            Ok(state)
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        state.send_message(status_error(format!("ps:timeseries:server:* ~ {}", e.to_string())));
                                                                        Err(e)
                                                                    }
                                                                }
                                                            }))
                                                        }
                                                }
                                            }).map_err(Into::<agent::Error>::into))
                                    },

                                    // Advance the internal iterator
                                    Ok(Command::Next) => {
                                        let (mut state2, maybe_chunk_iter) = state.split();
                                        if let Some(mut it) = maybe_chunk_iter {
                                            if let Some(chunk_bytes) = it.next() {
                                                if let Ok(bytes) = chunk_bytes {
                                                    debug!("sending message <NEXT::OK>");
                                                    state2.send_message(WsMessage::Binary(bytes));
                                                } else {
                                                    error!("sending message <NEXT::ERR>");
                                                    // TODO change malformed here to something more informative
                                                    state2.send_message(status_error("malformed byte payload"));
                                                }
                                            } else {
                                                info!("sending message <DONE>");
                                                state2.send_message(status_done());
                                            }
                                            f::ok(state2.join(Some(it))).into_trait()
                                        } else {
                                            info!("sending message <NOT-READY>");
                                            state2.send_message(status_not_ready());
                                            f::ok(state2).into_trait()
                                        }
                                    },

                                    // Close the iterator
                                    Ok(Command::Close) => {
                                        info!("sending message <CLOSE>");
                                        {
                                            state.send_message(status_done());
                                            state.close();
                                        }
                                        let t = state.kill().unwrap();
                                        f::ok(t).into_trait()
                                    }
                                }
                            })
                            .into_trait();

                        // Create a new Future that will run as long as each component
                        // future, `receiver` and `sender` are still running as well:
                        let f = read_client_commands
                            .map(|_| ());

                        let g = dispatch_client_commands
                            .map(|_| ());

                        let rx_kill = rx_kill
                            .map(|_| ())
                            .map_err(Into::<agent::Error>::into);

                        // When the kill signal is received, `rx_kill` will resolve.
                        let h = (f.select(g))
                            .map(|(item, _fut)| item)
                            .map_err(|(err, _fut)| err)
                            .select(rx_kill)
                            .map_err(move |(err, _fut)| {
                                error!("{:?} :: \n{}", id, err.to_string());
                            })
                            .and_then(|_| {
                                info!("websocket connection closed");
                                Ok(())
                            });

                        Arbiter::spawn(h);

                        Ok(())
                    })
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            })
            .or_else(|e| Err(Into::<agent::Error>::into(e)))
            .into_trait()
    }
}
