use nix::sys::utsname::UtsName;
use systemstat::{Ipv4Addr, Memory};

use crate::system::Uptime;

// #[derive(Debug)]
pub struct State {
    pub hi_count: usize,
    pub ip: Option<Ipv4Addr>,
    pub os_name: String,
    pub uname: UtsName,
    pub memory: Option<Memory>,
    pub uptime: Uptime,
}
