use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::raw::{c_int, c_long, c_void};
use std::time::{Duration, Instant};

use crate::icmp::{parse_echo_reply, Reply};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("This example currently supports Linux and macOS only.");

const AF_INET: c_int = 2;
const SOCK_RAW: c_int = 3;
const IPPROTO_ICMP: c_int = 1;

#[cfg(target_os = "linux")]
const SOL_SOCKET: c_int = 1;
#[cfg(target_os = "linux")]
const SO_RCVTIMEO: c_int = 20;

#[cfg(target_os = "macos")]
const SOL_SOCKET: c_int = 0xffff;
#[cfg(target_os = "macos")]
const SO_RCVTIMEO: c_int = 0x1006;

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn setsockopt(
        socket: c_int,
        level: c_int,
        option_name: c_int,
        option_value: *const c_void,
        option_len: SockLen,
    ) -> c_int;
    fn sendto(
        socket: c_int,
        buffer: *const c_void,
        length: usize,
        flags: c_int,
        destination_addr: *const SockAddr,
        dest_len: SockLen,
    ) -> isize;
    fn recvfrom(
        socket: c_int,
        buffer: *mut c_void,
        length: usize,
        flags: c_int,
        address: *mut SockAddr,
        address_len: *mut SockLen,
    ) -> isize;
}

type SockLen = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct InAddr {
    s_addr: u32,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct SockAddr {
    sa_len: u8,
    sa_family: u8,
    sa_data: [u8; 14],
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct SockAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrIn {
    sin_len: u8,
    sin_family: u8,
    sin_port: u16,
    sin_addr: InAddr,
    sin_zero: [u8; 8],
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: InAddr,
    sin_zero: [u8; 8],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct TimeVal {
    tv_sec: c_long,
    tv_usec: c_int,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct TimeVal {
    tv_sec: c_long,
    tv_usec: c_long,
}

impl SockAddrIn {
    fn new(ip: Ipv4Addr) -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                sin_len: std::mem::size_of::<Self>() as u8,
                sin_family: AF_INET as u8,
                sin_port: 0,
                sin_addr: InAddr {
                    s_addr: u32::from_ne_bytes(ip.octets()),
                },
                sin_zero: [0; 8],
            }
        }

        #[cfg(target_os = "linux")]
        {
            Self {
                sin_family: AF_INET as u16,
                sin_port: 0,
                sin_addr: InAddr {
                    s_addr: u32::from_ne_bytes(ip.octets()),
                },
                sin_zero: [0; 8],
            }
        }
    }

    fn empty() -> Self {
        Self::new(Ipv4Addr::UNSPECIFIED)
    }

    fn len(&self) -> SockLen {
        std::mem::size_of::<Self>() as SockLen
    }

    fn as_ptr(&self) -> *const SockAddr {
        self as *const Self as *const SockAddr
    }

    fn as_mut_ptr(&mut self) -> *mut SockAddr {
        self as *mut Self as *mut SockAddr
    }

    fn ip(&self) -> Ipv4Addr {
        Ipv4Addr::from(self.sin_addr.s_addr.to_ne_bytes())
    }
}

pub struct RawSocket {
    fd: OwnedFd,
}

impl RawSocket {
    pub fn new() -> io::Result<Self> {
        let fd = unsafe { socket(AF_INET, SOCK_RAW, IPPROTO_ICMP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Self { fd })
    }

    pub fn send_to(&self, ip: Ipv4Addr, packet: &[u8]) -> io::Result<()> {
        let address = SockAddrIn::new(ip);
        let written = unsafe {
            sendto(
                self.fd.as_raw_fd(),
                packet.as_ptr().cast(),
                packet.len(),
                0,
                address.as_ptr(),
                address.len(),
            )
        };

        if written < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub fn receive_reply(
        &self,
        identifier: u16,
        sequence: u16,
        timeout: Duration,
    ) -> io::Result<(Reply, IpAddr)> {
        loop {
            let (packet, from) = self.receive_packet(timeout)?;
            if let Some(reply) = parse_echo_reply(&packet) {
                if reply.identifier == identifier && reply.sequence == sequence {
                    return Ok((reply, from));
                }
            }
        }
    }

    pub fn receive_packet(&self, timeout: Duration) -> io::Result<(Vec<u8>, IpAddr)> {
        let deadline = Instant::now() + timeout;
        let mut buffer = [0u8; 1500];

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "request timed out"));
            }

            self.set_recv_timeout(remaining)?;

            let mut address = SockAddrIn::empty();
            let mut address_len = address.len();
            let received = unsafe {
                recvfrom(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    0,
                    address.as_mut_ptr(),
                    &mut address_len,
                )
            };

            if received < 0 {
                return Err(io::Error::last_os_error());
            }

            let received = received as usize;
            return Ok((buffer[..received].to_vec(), IpAddr::V4(address.ip())));
        }
    }

    fn set_recv_timeout(&self, timeout: Duration) -> io::Result<()> {
        let timeout = duration_to_timeval(timeout);
        let result = unsafe {
            setsockopt(
                self.fd.as_raw_fd(),
                SOL_SOCKET,
                SO_RCVTIMEO,
                (&timeout as *const TimeVal).cast(),
                std::mem::size_of::<TimeVal>() as SockLen,
            )
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

pub fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

fn duration_to_timeval(duration: Duration) -> TimeVal {
    #[cfg(target_os = "macos")]
    {
        TimeVal {
            tv_sec: duration.as_secs() as c_long,
            tv_usec: duration.subsec_micros() as c_int,
        }
    }

    #[cfg(target_os = "linux")]
    {
        TimeVal {
            tv_sec: duration.as_secs() as c_long,
            tv_usec: duration.subsec_micros() as c_long,
        }
    }
}
