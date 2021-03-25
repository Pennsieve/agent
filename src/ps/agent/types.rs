//! Agent-proxy specific types

use std::any::Any;
use std::cell::RefCell;
use std::str::FromStr;
use std::string::ToString;
use std::{fmt, result};

use anymap::{any, Map};

use actix::dev::*;

use url::Url;

use futures::future::Future as _Future;
use futures::sync::oneshot;
use futures::{self, future};

pub use crate::ps::agent::error::{Error, ErrorKind, Result};
use crate::ps::agent::messages::{ServerStartup, WorkerStartup};

pub type Future<T> = Box<dyn _Future<Item = T, Error = Error> + Send>;
pub type TxStop = oneshot::Sender<()>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A typeful representation of hostnames, allowing a String to be lifted
/// into a typed "http", "https", "ws", or "wss"-schemed hostname
pub enum HostName {
    Http(String),
    Https(String),
    Ws(String),
    Wss(String),
}

impl HostName {
    pub fn parse(s: &str) -> Result<Self> {
        s.parse()
    }

    pub fn scheme(&self) -> &str {
        match *self {
            HostName::Http(_) => "http",
            HostName::Https(_) => "https",
            HostName::Ws(_) => "ws",
            HostName::Wss(_) => "wss",
        }
    }
}

impl fmt::Display for HostName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

// https://bryce.fisher-fleig.org/blog/strategies-for-returning-references-in-rust/index.html
impl From<HostName> for String {
    fn from(hostname: HostName) -> Self {
        match hostname {
            HostName::Http(host) => format!("http://{}", host),
            HostName::Https(host) => format!("https://{}", host),
            HostName::Ws(host) => format!("ws://{}", host),
            HostName::Wss(host) => format!("wss://{}", host),
        }
    }
}

impl<'a> From<&'a HostName> for &'a String {
    fn from(hostname: &'a HostName) -> Self {
        hostname.into()
    }
}

/// Checks and normalizes a hostname.
///
/// # Examples
///
/// assert_eq!("http://127.0.0.1".parse::<HostName>(), Ok(HostName::Http("127.0.0.1")));
///
/// assert_eq!("http://google.com".parse::<HostName>(), Ok(HostName::Http("google.com")));
///
/// assert_eq!("ws://google.com".parse::<HostName>(), Ok(HostName::Ws("google.com")));
///
/// assert_eq!("google.com".parse::<HostName>(), Err(HostNameError::MalformedHostNameError(_)));
impl FromStr for HostName {
    type Err = Error;

    fn from_str(hostname: &str) -> result::Result<Self, Self::Err> {
        match Url::parse(hostname) {
            Err(_parse_err) => Err(Error::malformed_hostname(hostname)),
            Ok(url) => match url.host_str() {
                Some(host) => match url.scheme() {
                    "http" => Ok(HostName::Http(host.into())),
                    "https" => Ok(HostName::Https(host.into())),
                    "ws" => Ok(HostName::Ws(host.into())),
                    "wss" => Ok(HostName::Wss(host.into())),
                    scheme => Err(Error::unsupported_scheme(hostname, scheme)),
                },
                None => Err(Error::malformed_hostname(hostname)),
            },
        }
    }
}

/// A type that identifies a server implementation.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct ServiceId(pub &'static str);

impl From<ServiceId> for String {
    fn from(id: ServiceId) -> Self {
        id.0.into()
    }
}

impl<'a> From<ServiceId> for &'a str {
    fn from(id: ServiceId) -> Self {
        id.0
    }
}

impl ToString for ServiceId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

/// Output type formats
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Simple, // Simple, uncolorized newline separated text
    Rich,   // The default (colorized, terminal library supported IO)
            //Json, // JSON formatted
}

impl OutputFormat {
    #[allow(dead_code)]
    /// Tests if the output format is "simple".
    pub fn is_simple(self) -> bool {
        self == OutputFormat::Simple
    }

    #[allow(dead_code)]
    /// Tests if the output format is "rich".
    pub fn is_rich(self) -> bool {
        self == OutputFormat::Rich
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        #[cfg(windows)]
        {
            OutputFormat::Simple
        }
        #[cfg(not(windows))]
        {
            OutputFormat::Rich
        }
    }
}

impl FromStr for OutputFormat {
    type Err = Error;

    fn from_str(format: &str) -> result::Result<Self, Self::Err> {
        match format.to_lowercase().as_ref() {
            "rich" => Ok(OutputFormat::Rich),
            "simple" => Ok(OutputFormat::Simple),
            _ => Err(Error::output_format(format)),
        }
    }
}

/// A handle representing a running service. The handle is used to interact
/// with the service, which in the case of the agent, is a type that implements
/// the `Server` or `Worker` trait. The handle is used to start the service
/// and fetch the actix actor address associated with it upon its creation.
pub struct ServiceHandle {
    id: ServiceId,
    runner: Option<Box<dyn FnMut()>>,
    // We need to cast to any as the `Addr<_>` instance we get back from
    // `Actor.start()` will be parameterized by the actor type it was called
    // on. To normalize the type signature, we upcast to `Any`, and downcast
    // (safely) to get back to the original `Addr<T>` instance. Given the
    // value of `ServiceId`, it is the responsibility of the caller to
    // provide the type for downcasting.
    addr: Box<dyn Any + Send + 'static>,
}

impl ServiceHandle {
    pub fn new<A, F>(id: ServiceId, runner: F, addr: Addr<A>) -> Self
    where
        A: Actor,
        F: FnMut() + 'static,
    {
        Self {
            id,
            runner: Some(Box::new(runner)),
            addr: Box::new(addr),
        }
    }

    #[allow(dead_code)]
    /// Get the ID of the service.
    pub fn id(&self) -> &ServiceId {
        &self.id
    }

    /// Attempt to access the service's actor address as type `Addr<T>`.
    /// Note: The onus is on the caller to supply the concrete type of the
    /// Actor when accessing its address.
    pub fn as_addr<T>(&self) -> Option<&Addr<T>>
    where
        T: Actor,
    {
        self.addr.downcast_ref::<Addr<T>>()
    }

    /// Starts the service (server or worker).
    ///
    /// # Panic
    ///
    /// This function will panic if called more than once.
    pub fn run(&mut self) {
        self.runner
            .take()
            .unwrap_or_else(|| panic!("service {:?} already running", self.id()))()
    }
}

// Property map ("props") type shorthand:
//
// rustc complains according to a linter rule (as of 1.32),
// but according to https://github.com/chris-morgan/anymap/issues/31
// (dated 2018-09-17), it doesn't seem to be a hard-stopping issue and the core
// devs of rustc are aware.
type PROPMAP = Map<dyn any::CloneAny + Send + Sync>;

/// An environment that holds the "props" associated with servers and workers,
/// both of which are actix actors.
///
/// The props can be thought of as persistent initialization arguments that are
/// pinned to the environment a worker or server runs in. This behavior
/// is required for two reasons:
///
/// 1. In order to be able to look up a worker/server by its type in the
///    actix actor registry a la
///
///    `System::current().registry().get::<T>()`, where T: Actor + SystemService
///    ...
///    where SystemService: Actor + Supervised + Default
///
///    our `Server` and `Worker` implementations must also implement `Default`,
///    which dictates it must be instantiable without arguments.
///
///  2. If an actor dies, it must be restarted. This is the responsibility of
///     the running actix `System`. Again, no constructor arguments
///     are able to be passed into the actor instance.
///
///  In practice, this restriction is not terrible to work around, as
///  initialization arguments are stable, and once set do not change.
///
///  They are intended to be specified via `Agent::define_server` and
///  `Agent::define_worker`.
thread_local! {
    static ENVIRONMENT: RefCell<PROPMAP> = RefCell::new(Map::new())
}

/// Any `Server` or `Worker` that implements this trait and provides a prop
/// type can inject value of `Self::Props into its environment and fetch it
/// later.
pub trait WithProps: Actor {
    /// All prop types must be upcastable to `Any`, sized, sendable across
    /// thread boundaries, and cloneable.
    type Props: Any + Clone + Send + Sync + Sized;

    /// Lift the specified props into the environment.
    ///
    /// Note: This is not intended to be called from within any actor code.
    /// It is currently called by the `Server` and `Worker` initialization
    /// methods `Agent::define_server` and `Agent::define_worker`.
    fn with_props(props: Self::Props) {
        ENVIRONMENT.with(|e| {
            e.borrow_mut().insert(props);
        });
    }

    /// Gets a cloned instance of `Self::Props`.
    fn get_props(&self) -> Option<Self::Props> {
        ENVIRONMENT.with(|e| e.borrow().get::<Self::Props>().cloned())
    }

    /// Borrows props from the environment, passing a reference to said props
    /// into a provided closure where the reference is guaranteed to be valid
    /// for the lifetime of the props. The return value of the closure will
    /// be the return value of the `Props::borrow_props` method.
    fn borrow_props<F, T>(&self, scope: F) -> T
    where
        F: Fn(Option<&Self::Props>) -> T,
    {
        ENVIRONMENT.with(|e| {
            let r = e.borrow();
            let props: Option<&Self::Props> = r.get::<Self::Props>();
            scope(props)
        })
    }
}

/// A trait that defines an abstract service that can be run on a background
/// thread.
pub trait Service: Send + Sync {
    fn id(&self) -> ServiceId;
    fn run(self: Box<Self>) -> Result<ServiceHandle>;
}

/// An interface for any type that defines a server.
///
/// A "server" as we define it, is a background process that listens for and
/// responds to external communication, i.e. a HTTP server, etc.
///
/// # Example
///
///   `ReverseProxyServer`, `TimeSeriesServer`, and `StatusServer`.
///
pub trait Server:
    Send + Sync + Actor + Default + WithProps + Supervised + SystemService + Handler<ServerStartup>
{
    fn id(&self) -> ServiceId;
}

/// An interface for any type that defines a background worker.
///
/// A "worker" as we define it, is a non-interactive background process. It
/// performs a task--possibly once, possibly on a recurrent basis--without
/// interacting with a user.
///
/// # Example
///
///   `CachePageCollector`, `UploadWatcher`, and `Uploader`
///
pub trait Worker:
    Send + Sync + Actor + Default + WithProps + Supervised + SystemService + Handler<WorkerStartup>
{
    fn id(&self) -> ServiceId;
}

// Newtype style wrapper around futures needed to implement functionality for
// returning a future from an `actix::Handler::handle` method.
pub struct ServiceFuture<T> {
    inner: Future<T>,
}

impl<T> ServiceFuture<T> {
    pub fn wrap<F>(fut: F) -> ServiceFuture<T>
    where
        F: future::Future<Item = T, Error = Error> + Send + 'static,
    {
        Self {
            inner: Box::new(fut),
        }
    }
}

impl<T> future::Future for ServiceFuture<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

impl<A, M, I: 'static> MessageResponse<A, M> for ServiceFuture<I>
where
    A: Actor,
    M::Result: Send,
    M: Message<Result = Result<I>>,
    A::Context: AsyncContext<A>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        Arbiter::spawn(self.then(move |res| {
            if let Some(tx) = tx {
                tx.send(res)
            }
            Ok(())
        }));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hostname_parse_test_ok_1() {
        let result = "http://127.0.0.1".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HostName::Http("127.0.0.1".into()));
    }

    #[test]
    fn hostname_parse_test_ok_2() {
        let result = "https://127.0.0.1".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HostName::Https("127.0.0.1".into()));
    }

    #[test]
    fn hostname_parse_test_ok_3() {
        let result = "ws://127.0.0.1".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HostName::Ws("127.0.0.1".into()));
    }

    #[test]
    fn hostname_parse_test_ok_31() {
        let result = "ws://us-west-1.127.0.0.1".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HostName::Ws("us-west-1.127.0.0.1".into()));
    }

    #[test]
    fn hostname_parse_test_ok_4_preserve_domains() {
        let result = "http://us-west-1.foo.bar.baz.com?foo=bar&baz".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            HostName::Http("us-west-1.foo.bar.baz.com".into())
        );
    }

    #[test]
    fn hostname_parse_test_ok_5_extra_removed() {
        let result = "http://foo.com?foo=bar&baz".parse::<HostName>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HostName::Http("foo.com".into()));
    }

    #[test]
    fn hostname_parse_test_fail_1() {
        let hostname: String = "127.0.0.1".into();
        let result = hostname.parse::<HostName>();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::malformed_hostname(hostname));
    }

    #[test]
    fn hostname_parse_fail_3() {
        let hostname: String = "amqp://127.0.0.1".into();
        let result = hostname.parse::<HostName>();
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::unsupported_scheme(hostname, "amqp")
        );
    }

    #[test]
    fn hostname_parse_fail_4() {
        let hostname: String = "http://".into();
        let result = hostname.parse::<HostName>();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::malformed_hostname(hostname));
    }

    #[test]
    fn hostname_parse_fail_5() {
        let hostname: String = "".into();
        let result = hostname.parse::<HostName>();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::malformed_hostname(hostname));
    }

    #[test]
    fn hostname_parse_fail_6() {
        let hostname: String = "://".into();
        let result = hostname.parse::<HostName>();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::malformed_hostname(hostname));
    }
}
