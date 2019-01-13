use askama::Template;
use futures::{future, Future, Stream};
use hyper::{Body, Method, Request, Response, StatusCode};
use nix::sys::utsname::UtsName;
use std::sync::{Arc, RwLock};
use systemstat::Memory;

use crate::app::State;
use crate::system::Uptime;

static NOT_FOUND: &[u8] = b"Not found\n";

#[derive(Template)]
#[template(path = "hi.txt")]
pub struct HelloTemplate<'a> {
    hi_count: usize,
    ip: &'a str,
    os_name: &'a str,
    uname: &'a UtsName,
    memory: &'a Option<Memory>,
    uptime: &'a Uptime,
}

pub fn handle_request(
    state: Arc<RwLock<State>>,
    req: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/")
        | (&Method::GET, "/hi")
        | (&Method::HEAD, "/")
        | (&Method::HEAD, "/hi") => {
            let state = state.read().expect("poisioned"); // FIXME: Deal with this

            // FIXME: Don't do this everytime
            let ip_string = state
                .ip
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "?.?.?.?".to_string());

            let template = HelloTemplate {
                hi_count: state.hi_count,
                ip: &ip_string,
                memory: &state.memory,
                uptime: &state.uptime,
                os_name: &state.os_name,
                uname: &state.uname,
            };

            let response_data = template
                .render()
                .ok()
                .unwrap_or_else(|| "Internal Server Error\n".to_string());
            // FIXME: Set HTTP status
            Box::new(future::ok(Response::new(response_data.into())))
        }
        (&Method::POST, "/hi") => {
            Box::new(req.into_body().concat2().map(move |_b| {
                // Increment the hi count
                let body = {
                    let mut state = state.write().expect("poisioned");
                    state.hi_count += 1;

                    format!("Hello!, your number is {}\n", state.hi_count)
                };
                Response::new(body.into())
            }))
        }
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOT_FOUND.into())
                .unwrap(),
        )),
    }
}
