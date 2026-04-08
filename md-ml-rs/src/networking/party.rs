// By Boshi Yuan (Rust rewrite)

use crate::share::ShareElement;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

const RETRY_AFTER_SECONDS: u64 = 2;

pub struct Party {
    my_id: usize,
    num_parties: usize,
    send_streams: Vec<Option<TcpStream>>,
    receive_streams: Vec<Option<TcpStream>>,
    bytes_sent: AtomicU64,
}

impl Party {
    pub fn new(my_id: usize, num_parties: usize, port_base: usize) -> Self {
        let mut send_streams: Vec<Option<TcpStream>> = (0..num_parties).map(|_| None).collect();
        let mut receive_streams: Vec<Option<TcpStream>> = (0..num_parties).map(|_| None).collect();

        // Start listeners for accepting connections from other parties
        let listeners: Vec<Option<TcpListener>> = (0..num_parties)
            .map(|from_id| {
                if from_id == my_id {
                    None
                } else {
                    let port = Self::which_port(port_base, from_id, my_id, num_parties);
                    let addr = format!("127.0.0.1:{}", port);
                    Some(TcpListener::bind(&addr).unwrap_or_else(|e| {
                        panic!("Failed to bind to {}: {}", addr, e)
                    }))
                }
            })
            .collect();

        // Accept and connect in parallel using threads
        let mut handles = Vec::new();

        // Spawn accept threads
        for from_id in 0..num_parties {
            if from_id == my_id {
                continue;
            }
            let listener = listeners[from_id].as_ref().unwrap().try_clone().unwrap();
            let handle = thread::spawn(move || -> (usize, TcpStream) {
                let (stream, _) = listener.accept().expect("Failed to accept connection");
                stream.set_nodelay(true).ok();
                (from_id, stream)
            });
            handles.push(("recv", handle));
        }

        // Spawn connect threads
        for to_id in 0..num_parties {
            if to_id == my_id {
                continue;
            }
            let port = Self::which_port(port_base, my_id, to_id, num_parties);
            let handle = thread::spawn(move || -> (usize, TcpStream) {
                let addr = format!("127.0.0.1:{}", port);
                loop {
                    match TcpStream::connect(&addr) {
                        Ok(stream) => {
                            stream.set_nodelay(true).ok();
                            return (to_id, stream);
                        }
                        Err(_) => {
                            eprintln!(
                                "Failed to connect to party {}, retry after {} seconds...",
                                to_id, RETRY_AFTER_SECONDS
                            );
                            thread::sleep(Duration::from_secs(RETRY_AFTER_SECONDS));
                        }
                    }
                }
            });
            handles.push(("send", handle));
        }

        // Collect results
        for (kind, handle) in handles {
            let (id, stream) = handle.join().expect("Thread panicked");
            match kind {
                "recv" => receive_streams[id] = Some(stream),
                "send" => send_streams[id] = Some(stream),
                _ => unreachable!(),
            }
        }

        Party {
            my_id,
            num_parties,
            send_streams,
            receive_streams,
            bytes_sent: AtomicU64::new(0),
        }
    }

    fn which_port(port_base: usize, from_id: usize, to_id: usize, num_parties: usize) -> u16 {
        let ret = port_base + from_id * num_parties + to_id;
        assert!(ret <= 65535, "Port number exceeds 65535");
        ret as u16
    }

    fn check_id(&self, target_id: usize) {
        assert!(target_id < self.num_parties, "Target ID out of range");
        assert!(target_id != self.my_id, "Party cannot send to itself");
    }

    pub fn my_id(&self) -> usize {
        self.my_id
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    pub fn add_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn send<T: ShareElement>(&mut self, to_id: usize, message: T) {
        self.check_id(to_id);
        let bytes = message.to_le_bytes_vec();
        let stream = self.send_streams[to_id].as_mut().unwrap();
        stream.write_all(&bytes).expect("Failed to send");
        self.bytes_sent.fetch_add(bytes.len() as u64, Ordering::Relaxed);
    }

    pub fn receive<T: ShareElement>(&mut self, from_id: usize) -> T {
        self.check_id(from_id);
        let mut buf = vec![0u8; T::byte_size()];
        let stream = self.receive_streams[from_id].as_mut().unwrap();
        stream.read_exact(&mut buf).expect("Failed to receive");
        T::from_le_bytes(&buf)
    }

    pub fn send_vec<T: ShareElement>(&mut self, to_id: usize, message: &[T]) {
        self.check_id(to_id);
        // Zero-copy: view the element slice directly as bytes
        let bytes = T::slice_as_bytes(message);
        let stream = self.send_streams[to_id].as_mut().unwrap();
        // Write in chunks for large messages
        const CHUNK: usize = 1 << 20; // 1 MB
        for chunk in bytes.chunks(CHUNK) {
            stream.write_all(chunk).expect("Failed to send vec");
        }
        self.bytes_sent.fetch_add(bytes.len() as u64, Ordering::Relaxed);
    }

    pub fn receive_vec<T: ShareElement>(&mut self, from_id: usize, num_elements: usize) -> Vec<T> {
        self.check_id(from_id);
        let total_bytes = num_elements * T::byte_size();
        let mut buf = vec![0u8; total_bytes];
        let stream = self.receive_streams[from_id].as_mut().unwrap();
        stream.read_exact(&mut buf).expect("Failed to receive vec");
        T::vec_from_bytes(&buf)
    }

    pub fn send_vec_to_other<T: ShareElement>(&mut self, message: &[T]) {
        let other = 1 - self.my_id;
        self.send_vec(other, message);
    }

    pub fn receive_vec_from_other<T: ShareElement>(&mut self, num_elements: usize) -> Vec<T> {
        let other = 1 - self.my_id;
        self.receive_vec(other, num_elements)
    }

    /// Get mutable references to both send and receive streams for a given other party.
    /// This is needed when we want to send and receive concurrently using threads.
    pub fn split_streams(&mut self, other_id: usize) -> (&mut TcpStream, &mut TcpStream) {
        self.check_id(other_id);
        let send = self.send_streams[other_id].as_mut().unwrap() as *mut TcpStream;
        let recv = self.receive_streams[other_id].as_mut().unwrap() as *mut TcpStream;
        // Safety: send and receive streams are different objects
        unsafe { (&mut *send, &mut *recv) }
    }
}
