use nix::sys::utsname::UtsName;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use systemstat::{Ipv4Addr, Memory};

use crate::system::Uptime;

// #[derive(Debug)]
pub struct State {
    pub ip: Option<Ipv4Addr>,
    pub os_name: String,
    pub uname: UtsName,
    pub memory: Option<Memory>,
    pub uptime: Uptime,
    pub hellos: HashMap<IpAddr, Instant>,
}

impl State {
    pub fn new(
        ip: Option<Ipv4Addr>,
        os_name: String,
        uname: UtsName,
        memory: Option<Memory>,
        uptime: Uptime,
    ) -> Self {
        State {
            ip,
            os_name,
            uname,
            memory,
            uptime,
            hellos: HashMap::new(),
        }
    }

    pub fn inc_hi_count(&mut self, from: IpAddr) {
        self.hellos.insert(from, Instant::now());
    }

    pub fn hi_count(&self) -> usize {
        self.hellos.len()
    }
}
