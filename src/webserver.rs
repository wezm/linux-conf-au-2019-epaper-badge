use askama::Template;
use futures::{future, Future, Stream};
use hyper::{header, Body, Method, Request, Response, StatusCode};
use memmem::{Searcher, TwoWaySearcher};
use nix::sys::utsname::UtsName;
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use systemstat::Memory;

use crate::app::State;
use crate::system::Uptime;

static NOT_FOUND: &[u8] = b"Not found\n";

#[derive(Template)]
#[template(path = "hi.txt")]
pub struct HelloTextTemplate<'a> {
    hi_count: usize,
    ip: &'a str,
    os_name: &'a str,
    uname: &'a UtsName,
    memory: &'a Option<Memory>,
    uptime: &'a Uptime,
}

#[derive(Template)]
#[template(path = "hi.html")]
pub struct HelloHtmlTemplate<'a> {
    hi_count: usize,
    os_name: &'a str,
    uname: &'a UtsName,
    memory: &'a Option<Memory>,
    uptime: &'a Uptime,
}

pub fn handle_request(
    state: Arc<RwLock<State>>,
    remote_addr: SocketAddr,
    req: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/")
        | (&Method::GET, "/hi")
        | (&Method::HEAD, "/")
        | (&Method::HEAD, "/hi") => {
            let state = state.read().expect("poisioned"); // FIXME: Deal with this

            // See if the user agent supports html
            let accepts_html = req
                .headers()
                .get(header::ACCEPT)
                .and_then(|accept| TwoWaySearcher::new(b"text/html").search_in(accept.as_bytes()))
                .is_some();
            let response_data = if accepts_html {
                let template = HelloHtmlTemplate {
                    hi_count: state.hi_count(),
                    memory: &state.memory,
                    uptime: &state.uptime,
                    os_name: &state.os_name,
                    uname: &state.uname,
                };

                template
                    .render()
                    .ok()
                    .unwrap_or_else(|| "Internal Server Error\n".to_string())
            } else {
                // FIXME: Don't do this everytime
                let ip_string = state
                    .ip
                    .map(|ip| ip.to_string())
                    .unwrap_or_else(|| "?.?.?.?".to_string());

                let template = HelloTextTemplate {
                    hi_count: state.hi_count(),
                    ip: &ip_string,
                    memory: &state.memory,
                    uptime: &state.uptime,
                    os_name: &state.os_name,
                    uname: &state.uname,
                };

                template
                    .render()
                    .ok()
                    .unwrap_or_else(|| "Internal Server Error\n".to_string())
            };

            // FIXME: Set HTTP status
            Box::new(future::ok(Response::new(response_data.into())))
        }
        (&Method::POST, "/hi") => {
            Box::new(
                req.into_body()
                    .fold((), |_, _chunk| future::ok::<_, hyper::Error>(()))
                    .map(move |()| {
                        // Increment the hi count
                        let body = {
                            let mut state = state.write().expect("poisioned");
                            state.inc_hi_count(remote_addr.ip());

                            format!(
                                "Hello! You're the {} person to say hi.\n",
                                Ordinal(state.hi_count())
                            )
                        };
                        Response::new(body.into())
                    }),
            )
        }
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOT_FOUND.into())
                .unwrap(),
        )),
    }
}

struct Ordinal(usize);

impl fmt::Display for Ordinal {
    // Inspited by activesupport/lib/active_support/inflector/methods.rb
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let ordinal = if (11..=13).includes(self.0 % 100) {
            "th"
        } else {
            match self.0 % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            }
        };

        f.write_str(&self.0.to_string())?;
        f.write_str(ordinal)
    }
}

trait Includes {
    type Item: std::cmp::PartialOrd;
    fn includes(&self, item: Self::Item) -> bool;
}

impl<I: std::cmp::PartialOrd> Includes for std::ops::RangeInclusive<I> {
    type Item = I;

    fn includes(&self, item: Self::Item) -> bool {
        item >= *self.start() && item <= *self.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordinal() {
        assert_eq!(Ordinal(0).to_string(), String::from("0th"));
        assert_eq!(Ordinal(1).to_string(), String::from("1st"));
        assert_eq!(Ordinal(2).to_string(), String::from("2nd"));
        assert_eq!(Ordinal(3).to_string(), String::from("3rd"));
        assert_eq!(Ordinal(4).to_string(), String::from("4th"));
        assert_eq!(Ordinal(5).to_string(), String::from("5th"));
        assert_eq!(Ordinal(6).to_string(), String::from("6th"));
        assert_eq!(Ordinal(7).to_string(), String::from("7th"));
        assert_eq!(Ordinal(8).to_string(), String::from("8th"));
        assert_eq!(Ordinal(9).to_string(), String::from("9th"));
        assert_eq!(Ordinal(10).to_string(), String::from("10th"));
        assert_eq!(Ordinal(11).to_string(), String::from("11th"));
        assert_eq!(Ordinal(12).to_string(), String::from("12th"));
        assert_eq!(Ordinal(13).to_string(), String::from("13th"));
        assert_eq!(Ordinal(22).to_string(), String::from("22nd"));
        assert_eq!(Ordinal(33).to_string(), String::from("33rd"));
        assert_eq!(Ordinal(44).to_string(), String::from("44th"));
        assert_eq!(Ordinal(100).to_string(), String::from("100th"));
        assert_eq!(Ordinal(1300).to_string(), String::from("1300th"));
        assert_eq!(Ordinal(1533).to_string(), String::from("1533rd"));
    }
}
