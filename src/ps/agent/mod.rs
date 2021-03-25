//! The Pennsieve Agent implementation

pub mod api;
pub mod cache;
pub mod cli;
pub mod config;
pub mod database;
pub mod error;
pub mod features;
pub mod messages;
pub mod server;
pub mod types;
pub mod upload;
pub mod version;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;

use actix::dev::*;
use log::*;
use log_mdc;

use self::messages::{ServerStartup, WorkerStartup};
use self::types::ServiceHandle;
pub use self::types::{
    Error, ErrorKind, Future, HostName, OutputFormat, Result, Server, Service, ServiceFuture,
    ServiceId, WithProps, Worker,
};

// A simple macro that sets up logging for background services.
macro_rules! setup_logging {
    () => {{
        let mut mdc = vec![];
        log_mdc::iter(|k, v| mdc.push((k.to_owned(), v.to_owned())));
        mdc.push(("tid".to_owned(), format!("{:?}", thread::current().id())));
        log_mdc::extend(mdc);
    };};
}

/// Upon starting the agent, an `AgentHandle` instance is returned.
/// The `AgentHandle` is used to communicate with the running agent and its
/// respective child services, including termination of the agent.
pub struct AgentHandle {
    handles: Vec<ServiceHandle>,
    status_addr: Option<Addr<server::StatusServer>>,
    #[allow(dead_code)]
    status_port: u16,
    #[allow(dead_code)]
    quiet: bool,
}

impl AgentHandle {
    /// Create a handle to the running agent.
    fn new(handles: Vec<ServiceHandle>, status_port: u16, quiet: bool) -> Self {
        Self {
            handles,
            status_addr: None,
            status_port,
            quiet,
        }
    }

    /// Attempt to look up the address of a service by its type.
    ///
    /// Note: If multiple services of the same type are registered, only the
    /// first instance will be returned (if it exists).
    pub fn lookup_addr<T>(&self) -> Option<Addr<T>>
    where
        T: Actor,
    {
        // TODO: it is difficult to store the `Add<_>` types in some kind
        // of associative collection because the inner type `_` differs per
        // actor. For right now, it's simpler to just iterate over the list
        // of service handles and attempt to downcast its container `Addr<_>`
        // into the proper type.
        for handle in self.handles.iter() {
            if let Some(addr) = handle.as_addr::<T>() {
                return Some(addr.clone());
            }
        }
        None
    }

    /// Return the address of the status actor.
    ///
    /// Note: The status actor will only be available after `AgentHandle.run()`
    /// is called.
    pub fn status_addr(&self) -> Option<&Addr<server::StatusServer>> {
        self.status_addr.as_ref()
    }

    /// Starts all the services (servers and workers) defined in the agent
    /// `$PENNSIEVE_HOME/config.ini` file.
    pub fn run(&mut self) -> Result<()> {
        // Start the status server:
        let status_addr = server::StatusServer::new().start();

        // Start up the services:
        for handle in self.handles.iter_mut() {
            handle.run();
        }

        #[cfg(not(debug_assertions))]
        {
            if !self.quiet {
                println!(
                    "Status server listening on port {port}",
                    port = self.status_port
                );
            }
        }

        // Tell the status server to start up the websocket frontend:
        status_addr.do_send(messages::StartStatusServer::new(self.status_port));
        self.status_addr = Some(status_addr);

        Ok(())
    }
}

/// A type that encodes a definition of a server to be run by the agent.
/// A server context contains the context necessary in order to start (and
/// restart) a service actor as needed.
struct ServerContext<S: Server> {
    local_port: u16,
    inner: S,
}

impl<S: Server> ServerContext<S> {
    /// Define a new server with associated props.
    pub fn define(local_port: u16, props: S::Props, inner: S) -> Self {
        S::with_props(props);
        Self { local_port, inner }
    }

    /// Take the server implementation contained in this context.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Server> Service for ServerContext<S> {
    fn id(&self) -> ServiceId {
        self.inner.id()
    }

    /// This function is responsible for starting the threads used to run the given
    /// `Server` instance
    fn run(self: Box<Self>) -> Result<ServiceHandle> {
        let id = self.id();
        let local_port = self.local_port;
        setup_logging!();
        let inner = self.into_inner();
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), local_port);
        let addr = inner.start();
        let addr_clone = addr.clone();
        Ok(ServiceHandle::new(
            id,
            move || addr.do_send(ServerStartup::new(address)),
            addr_clone,
        ))
    }
}

/// A type that encodes a definition of a server to be run by the agent.
struct WorkerContext<W: Worker> {
    inner: W,
}

impl<W: Worker> WorkerContext<W> {
    /// Define a new worker with associated props.
    pub fn define(props: W::Props, inner: W) -> Self {
        W::with_props(props);
        Self { inner }
    }

    /// Take the worker implementation contained in this context.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Worker> Service for WorkerContext<W> {
    fn id(&self) -> ServiceId {
        self.inner.id()
    }

    fn run(self: Box<Self>) -> Result<ServiceHandle> {
        let id = self.id();
        setup_logging!();
        let inner = self.into_inner();
        let addr = inner.start();
        let addr_clone = addr.clone();
        Ok(ServiceHandle::new(
            id,
            move || addr.do_send(WorkerStartup),
            addr_clone,
        ))
    }
}

#[derive(Default)]
pub struct Agent {
    // Track which ports are in use.
    ports_in_use: HashMap<u16, ServiceId>,
    // Definitions for the servers and workers that will be run by the agent:
    services: Vec<Box<dyn Service>>,
    // Status server port
    status_port: u16,
    // Supress output?
    quiet: bool,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            ports_in_use: HashMap::new(),
            services: vec![],
            quiet: false,
            status_port: config::constants::CONFIG_DEFAULT_STATUS_WEBSOCKET_PORT,
        }
    }

    /// Checks and registers a local port as in use by a specific server,
    /// returning an error if the specified port is already being used.
    fn check_and_register_port(&mut self, local_port: u16, id: ServiceId) -> Result<()> {
        if let Some(existing_server_id) = self.ports_in_use.get(&local_port) {
            let err = server::Error::port_already_in_use(local_port, *existing_server_id);
            return Err(err.into());
        }

        self.ports_in_use.insert(local_port, id);
        Ok(())
    }

    #[allow(dead_code)]
    /// Enable logging output.
    pub fn loud(&mut self) {
        self.quiet = false;
    }

    #[allow(dead_code)]
    /// Suppress logging output.
    pub fn quiet(&mut self) {
        self.quiet = true;
    }

    #[allow(dead_code)]
    /// Sets the port the status server will listen on.
    pub fn set_status_port(&mut self, port: u16) {
        self.status_port = port;
    }

    /// Defines a new server for the agent to run.
    pub fn define_server<S>(
        &mut self,
        local_port: u16,
        props: S::Props,
        server: S,
    ) -> Result<&mut Self>
    where
        S: 'static + Server,
    {
        match self.check_and_register_port(local_port, server.id()) {
            Ok(_) => {
                info!("Defined server: port {} => {:?}", local_port, server.id());
                self.services
                    .push(Box::new(ServerContext::define(local_port, props, server)));
                Ok(self)
            }
            Err(e) => Err(e),
        }
    }

    /// Defines a new background worker for the agent to run.
    ///
    /// Workers differ from Servers in that they are non-interactive and don't
    /// communicate with an outside client.
    pub fn define_worker<W>(&mut self, props: W::Props, worker: W) -> Result<&mut Self>
    where
        W: 'static + Worker,
    {
        info!("Defined worker => {:?}", worker.id());
        self.services
            .push(Box::new(WorkerContext::define(props, worker)));
        Ok(self)
    }

    /// Sets up the agent, returning a used to interact with it.
    pub fn setup(self) -> Result<AgentHandle> {
        #[cfg(not(debug_assertions))]
        let quiet = self.quiet;

        let handles: Vec<ServiceHandle> = self
            .services
            .into_iter()
            .map(|service| {
                let id = service.id();
                #[cfg(not(debug_assertions))]
                {
                    if !quiet {
                        println!("{name}: starting", name = id.to_string());
                    }
                }
                info!("{name}: starting", name = id.to_string());
                service.run()
            })
            .collect::<Result<Vec<ServiceHandle>>>()?;

        Ok(AgentHandle::new(handles, self.status_port, self.quiet))
    }
}

#[cfg(test)]
mod test {
    use crate::ps;
    use crate::ps::agent::{server, Agent};
    use actix::prelude::*;

    const REMOTE_HOST: &str = "https://httpbin.org";

    #[test]
    fn server_requires_a_nonzero_nonprivileged_port() {
        let mut agent = Agent::new();
        let props = server::rp::Props {
            hostname: REMOTE_HOST.parse::<ps::HostName>().unwrap(),
            remote_port: 80,
        };
        System::run(|| {
            assert!(agent
                .define_server(8008, props, server::ReverseProxyServer)
                .is_ok());
            agent
                .setup()
                .expect("agent: setup")
                .run()
                .expect("agent: run");
            System::current().stop();
        });
    }

    #[test]
    fn agent_servers_disjoint_ports_ok() {
        let mut agent = Agent::new();
        let hostname = REMOTE_HOST.parse::<ps::HostName>().unwrap();
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 81,
            };
            assert!(agent
                .define_server(8888, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 82,
            };
            assert!(agent
                .define_server(8889, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 83,
            };
            assert!(agent
                .define_server(8890, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }
        System::run(|| {
            agent
                .setup()
                .expect("agent: setup")
                .run()
                .expect("agent: run");
            System::current().stop();
        });
    }

    #[test]
    fn agent_servers_overlapping_ports_fail() {
        let mut agent = Agent::new();
        let hostname = REMOTE_HOST.parse::<ps::HostName>().unwrap();
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 84,
            };
            assert!(agent
                .define_server(8888, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 85,
            };
            assert!(agent
                .define_server(8888, props.clone(), server::ReverseProxyServer)
                .is_err());
        }
        {
            let props = server::rp::Props {
                hostname: hostname.clone(),
                remote_port: 86,
            };
            assert!(agent
                .define_server(8890, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }
        assert!(agent.setup().is_ok());
    }

    #[test]
    fn agent_can_get_addr_for_worker() {
        let mut agent = Agent::new();
        let props = server::rp::Props {
            hostname: REMOTE_HOST.parse::<ps::HostName>().unwrap(),
            remote_port: 84,
        };
        {
            assert!(agent
                .define_server(8888, props.clone(), server::ReverseProxyServer)
                .is_ok());
        }

        let mut handle = agent.setup().expect("agent: setup");

        System::run(move || {
            handle.run().expect("agent: run");

            assert!(handle.lookup_addr::<server::ReverseProxyServer>().is_some());
            assert!(handle.lookup_addr::<server::StatusServer>().is_none());

            System::current().stop();
        });
    }
}
