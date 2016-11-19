use std::io;
use std::io::prelude::*;
use std::str;
use std::fmt;
use std::net::{Ipv4Addr, TcpStream, UdpSocket};
use std::time::{Instant, Duration};

#[derive(Debug,PartialEq,Clone)]
pub struct Board {
    pub ip: Ipv4Addr,
    pub mac: [u8; 6],
}

pub fn autodiscover() -> io::Result<Vec<Board>> {
    let mut boards: Vec<Board> = Vec::new();
    let mut buf = [0u8; 128];
    let socket = UdpSocket::bind(("0.0.0.0", 9090))?;
    socket.set_read_timeout(Some(Duration::from_millis(500)))?;
    let start = Instant::now();
    while Instant::now().duration_since(start) < Duration::from_millis(2000) {
        match socket.recv(&mut buf) {
            Ok(_) => match Board::from_packet(&buf) {
                Some(board) => if !boards.contains(&board) {
                    boards.push(board);
                },
                None => continue,
            },
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => continue,
                io::ErrorKind::TimedOut => continue,
                _ => return Err(e),
            },
        }
    }
    Ok(boards)
}

impl Board {
    fn from_packet(buf: &[u8]) -> Option<Board> {
        if buf.len() < 18 {
            return None;
        }

        match str::from_utf8(&buf[0..8]) {
            Ok(s) => if s != "PORTFIRE" { return None; },
            Err(_) => return None,
        }

        let ip = Ipv4Addr::new(buf[8], buf[9], buf[10], buf[11]);
        let mac = [buf[12], buf[13], buf[14], buf[15], buf[16], buf[17]];

        Some(Board { ip: ip, mac: mac })
    }

    fn txrx(&self, cmd: &[u8], buf: &mut [u8]) -> io::Result<usize> {
        let mut stream = TcpStream::connect((self.ip, 9090))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(1500)))?;
        stream.set_write_timeout(Some(Duration::from_millis(100)))?;
        let _ = stream.write(cmd)?;
        stream.read(buf)
    }

    fn txrx_ok(&self, cmd: &[u8]) -> io::Result<()> {
        let mut buf = [0u8; 4];
        self.txrx(cmd, &mut buf)?;
        if buf[0] == 'O' as u8 && buf[1] == 'K' as u8 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Invalid response"))
        }
    }

    pub fn ping(&self) -> io::Result<()> {
        let cmd = ['p' as u8];
        self.txrx_ok(&cmd)
    }

    pub fn arm(&self) -> io::Result<()> {
        let cmd = ['a' as u8];
        self.txrx_ok(&cmd)
    }

    pub fn disarm(&self) -> io::Result<()> {
        let cmd = ['d' as u8];
        self.txrx_ok(&cmd)
    }

    pub fn fire(&self, channels: [u8; 3]) -> io::Result<()> {
        let cmd = ['f' as u8, channels[0], channels[1], channels[2]];
        self.txrx_ok(&cmd)
    }

    pub fn bus_voltage(&self) -> io::Result<f32> {
        let mut buf = [0u8; 2];
        let cmd = ['b' as u8];
        self.txrx(&cmd, &mut buf)?;
        Ok((((buf[0] as u16) | ((buf[1] as u16)<<8)) as f32)/1000.0)
    }

    pub fn continuities(&self) -> io::Result<[u8; 31]> {
        let mut buf = [0u8; 31];
        let cmd = ['c' as u8];
        self.txrx(&cmd, &mut buf)?;
        Ok(buf)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Board {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} at {}",
               self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5],
               self.ip)
    }
}
