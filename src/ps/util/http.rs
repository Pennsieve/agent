//! Utility functions for use with Hyper

use hyper::{Body, Request, Response, StatusCode};
use log::*;

#[allow(dead_code)]
/// Generate a 50x HTTP response with the specified message
pub fn fail_with_message(reason: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(reason.into())
        .expect("couldn't create response")
}

#[allow(dead_code)]
/// Print the contents of the Request (including headers)
pub fn inspect_request(req: &Request<Body>) {
    info!("========================================");
    info!("{} {}", req.method(), req.uri());
    let headers = req.headers();
    for (name, value) in headers.iter() {
        info!("- {:?}: {:?}", name, value);
    }
}
