#[macro_use]
extern crate pennsieve_macros;

mod helpers;

use std::{thread, time};

use actix::prelude::*;

use serde_json::Value;

use pennsieve::{server, Agent, HostName};

const POST_JSON_DATA: &str = "{ \"foo\": \"bar\" }";

/// Tests in this file do not run on Windows because running/stopping
/// multiple systems in multiple threads seems to interfere with
/// actix's windows-specific signal handling streams.
///
/// We don't run more than one actor system in production anyway so as
/// long as these tests run on unix systems, we don't expect to see
/// any unexpected behavior on windows.

#[test]
#[cfg(unix)]
fn test_proxied_http_requests() {
    let system = System::new("ps");
    let props = server::rp::Props {
        hostname: "http://httpbin.org"
            .parse::<HostName>()
            .expect("single: parse"),
        remote_port: 80,
    };
    let local_port = 8090; //porthole::open().expect("couldn't find a free port");
    let local_uri = format!("127.0.0.1:{}", local_port);

    let mut agent = Agent::new();
    agent
        .define_server(local_port, props, server::ReverseProxyServer)
        .expect("server");

    let current = System::current();

    thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(200));

        // Request #1 - GET OK
        {
            let full_local_uri = format!("http://{}/get?foo=bar", local_uri);
            println!("single: start request 1 = {}", full_local_uri);
            let mut resp = reqwest::get(&full_local_uri).unwrap();
            assert!(resp.status().is_success());
            let json: Value = resp.json().unwrap();
            assert_eq!(&json["args"]["foo"], &Value::String("bar".into()));
            assert_eq!(&json["args"]["foo"], &Value::String("bar".into()));
            println!("single: finished request 1");
        }

        // Request #2 - GET Fail
        {
            let full_local_uri = format!("http://{}/this/route/does/not/exist", local_uri);
            let resp = reqwest::get(&full_local_uri).unwrap();
            assert!(resp.status().is_client_error());
            println!("single: finished request 2");
        }

        // Request #3 - POST OK
        {
            let full_local_uri = format!("http://{}/post", local_uri);
            let client = reqwest::Client::new();
            let mut resp = client
                .post(&full_local_uri)
                .body(POST_JSON_DATA)
                .send()
                .unwrap();
            assert!(resp.status().is_success());
            let json: Value = resp.json().unwrap();
            assert_eq!(&json["json"]["foo"], &Value::String("bar".into()));
            println!("single: finished request 3");
        }

        // Request #4 - POST Fail
        {
            let full_local_uri = format!("http://{}/this/route/does/not/exist", local_uri);
            let client = reqwest::Client::new();
            let resp = client
                .post(&full_local_uri)
                .body(POST_JSON_DATA)
                .send()
                .unwrap();
            assert!(resp.status().is_client_error());
            println!("single: finished request 4");
        }

        current.stop();
    });

    agent.setup().expect("setup").run().expect("run");
    system.run();
}

#[test]
#[cfg(unix)]
fn test_proxied_multiple_servers() {
    let system = System::new("ps");
    let props = server::rp::Props {
        hostname: "http://httpbin.org"
            .parse::<HostName>()
            .expect("multi: parse"),
        remote_port: 80,
    };

    let local_port_0 = 8091; //porthole::open().expect("couldn't find a free port");
    let local_port_1 = 8092; //porthole::open().expect("couldn't find a free port");
    let local_port_2 = 8093; //porthole::open().expect("couldn't find a free port");
    let local_port_3 = 8094; //porthole::open().expect("couldn't find a free port");

    let local_uri_0 = format!("http://127.0.0.1:{}", local_port_0);
    let local_uri_1 = format!("http://127.0.0.1:{}", local_port_1);
    let local_uri_2 = format!("http://127.0.0.1:{}", local_port_2);
    let local_uri_3 = format!("http://127.0.0.1:{}", local_port_3);

    let mut agent = Agent::new();
    agent
        .define_server(local_port_0, props.clone(), server::ReverseProxyServer)
        .expect("multi: server 0");
    agent
        .define_server(local_port_1, props.clone(), server::ReverseProxyServer)
        .expect("multi: server 1");
    agent
        .define_server(local_port_2, props.clone(), server::ReverseProxyServer)
        .expect("multi: server 2");
    agent
        .define_server(local_port_3, props.clone(), server::ReverseProxyServer)
        .expect("multi: server 3");

    let current = System::current();

    thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(200));

        // We should get responses from each:
        assert!(reqwest::get(local_uri_0.as_str())
            .expect("multi: response 1")
            .status()
            .is_success());
        println!("multi: finished request 1");
        assert!(reqwest::get(local_uri_1.as_str())
            .expect("multi: response 2")
            .status()
            .is_success());
        println!("multi: finished request 2");
        assert!(reqwest::get(local_uri_2.as_str())
            .expect("multi: response 3")
            .status()
            .is_success());
        println!("multi: finished request 3");
        assert!(reqwest::get(local_uri_3.as_str())
            .expect("multi: response 4")
            .status()
            .is_success());
        println!("multi: finished request 4");

        current.stop();
    });

    agent.setup().expect("setup").run().expect("run");

    system.run();
}
