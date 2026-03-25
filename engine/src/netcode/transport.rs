use ggrs::{Message, NonBlockingSocket};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

pub struct UdpNonBlockingSocket {
    socket: UdpSocket,
    addr_cache: HashMap<String, SocketAddr>,
}

impl UdpNonBlockingSocket {
    pub fn bind(port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            addr_cache: HashMap::new(),
        })
    }

    fn resolve(&mut self, addr: &str) -> SocketAddr {
        if let Some(&cached) = self.addr_cache.get(addr) {
            return cached;
        }
        let sock_addr: SocketAddr = addr.parse().expect("invalid socket address");
        self.addr_cache.insert(addr.to_string(), sock_addr);
        sock_addr
    }
}

impl NonBlockingSocket<String> for UdpNonBlockingSocket {
    fn send_to(&mut self, msg: &Message, addr: &String) {
        let sock_addr = self.resolve(addr);
        if let Ok(data) = bincode::serialize(msg) {
            let _ = self.socket.send_to(&data, sock_addr);
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        let mut messages = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
                        messages.push((src.to_string(), msg));
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        messages
    }
}
