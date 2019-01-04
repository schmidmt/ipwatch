use env_logger;

#[macro_use]
extern crate log;
extern crate failure;
extern crate netlink_packet;
extern crate netlink_proto;
extern crate netlink_sys;

use regex;
use core::time::Duration;
use subprocess;
use std::ffi::OsString;
use std::iter::FromIterator;
use futures::{future, Future, Stream};
use netlink_packet::{NetlinkMessage, RtnlMessage, NetlinkPayload, LinkNla, AddressNla};
use netlink_proto::{NetlinkCodec, NetlinkFramed};
use netlink_sys::constants::{RTMGRP_IPV4_IFADDR, RTMGRP_IPV6_IFADDR};
use netlink_sys::{Protocol, SocketAddr, TokioSocket};

use clap::{Arg, App};

use std::io;

fn changed_interface_stream() -> io::Result<Box<Stream<Item = String, Error = io::Error>>>
{
    let addr = SocketAddr::new(0, (RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR) as u32);
    let mut socket = TokioSocket::new(Protocol::Route)?;
    socket.bind(&addr)?;
    
    let stream = NetlinkFramed::new(socket, NetlinkCodec::<NetlinkMessage>::new())
        .map(|(packet, _addr)| {
            match packet.payload() {
                NetlinkPayload::Rtnl(msg) => {
                    match msg {
                        RtnlMessage::NewLink(lm) =>
                            lm.nlas().iter().find_map(|x| match x {
                                LinkNla::IfName(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::DelLink(lm) =>
                            lm.nlas().iter().find_map(|x| match x {
                                LinkNla::IfName(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::GetLink(lm) =>
                            lm.nlas().iter().find_map(|x| match x {
                                LinkNla::IfName(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::SetLink(lm) =>
                            lm.nlas().iter().find_map(|x| match x {
                                LinkNla::IfName(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::NewAddress(lm) =>
                            lm.nlas.iter().find_map(|x| match x {
                                AddressNla::Label(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::DelAddress(lm) =>
                            lm.nlas.iter().find_map(|x| match x {
                                AddressNla::Label(s) => Some(s.clone()),
                                _ => None
                            }),
                        RtnlMessage::GetAddress(lm) =>
                            lm.nlas.iter().find_map(|x| match x {
                                AddressNla::Label(s) => Some(s.clone()),
                                _ => None
                            }),
                    }
                },
                _ => None
            }
        })
        .filter_map(|x| x);

        Ok(Box::new(stream))
}

macro_rules! errorToIoError {
    ($e:expr) => {$e.map_err(|z| io::Error::new(io::ErrorKind::Other, z))};
}


fn start_process(command: &[OsString]) -> io::Result<subprocess::Popen> {
    let res = subprocess::Popen::create(command, subprocess::PopenConfig::default());
    errorToIoError!(res)
}

fn restart_process(command: &[OsString], current_process: &mut subprocess::Popen, timeout: Duration) -> io::Result<subprocess::Popen> {
    current_process.terminate()?;
    let exit_code = errorToIoError!(current_process.wait_timeout(timeout))?;
    if exit_code.is_none() {
        current_process.kill()?;
    }
    start_process(command)
}


fn main() -> io::Result<()> {
    env_logger::init();

    let matches = App::new("ipwatch")
        .arg(
            Arg::with_name("interface")
                .short("i")
                .long("interface")
                .value_name("IFNAME")
                .takes_value(true)
                .help("Interface to monitor using regexp (default .*)")
        )
        .arg(
            Arg::with_name("command")
                .help("Command to run")
                .required(true)
                .index(1)
                .multiple(true)
                .allow_hyphen_values(true)
        )
        .arg(
            Arg::with_name("timeout")
                .help("Specify the grace period between SIGTERM and SIGKILL (default 10)")
                .short("t")
                .long("timeout")
                .takes_value(true)
        )
        .get_matches();

    let interface_regexp = errorToIoError!(regex::Regex::new(matches.value_of("interface").unwrap_or(".*")))?;
    let command: Vec<OsString> = Vec::from_iter(matches.values_of_os("command").unwrap().map(|x| x.to_owned()));
    let timeout_str = matches.value_of("timeout").unwrap_or("10");
    let timeout: Duration = Duration::new(
        errorToIoError!(timeout_str.parse::<u64>())?,
        0
    );

    info!("Filtering to interface = {}", interface_regexp);
    info!("Command to run: {:?}", command);

    let stream = changed_interface_stream()?;
    let mut process = start_process(&command)?;

    stream
        .filter(|x| interface_regexp.is_match(x))
        .for_each(|x: String| {
            info!("Restarting process since interface {} changed", x);
            match restart_process(&command, &mut process, timeout) {
                Ok(new_process) => {
                    process = new_process;
                    future::ok(())
                },
                Err(e) => {
                    future::err(e)
                }
            }
        })
        .wait()
}
