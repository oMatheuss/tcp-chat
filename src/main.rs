use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::result;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

type Result<T> = result::Result<T, ()>;

enum ChatMessage {
    Connected(Arc<TcpStream>),
    Disconnected(Arc<TcpStream>),
    Message(Arc<TcpStream>, Vec<u8>),
}

struct Client {
    conn: Arc<TcpStream>,
}

fn client(sender: Sender<ChatMessage>, stream: Arc<TcpStream>) -> Result<()> {
    sender
        .send(ChatMessage::Connected(stream.clone()))
        .map_err(|err| eprintln!("ERROR: channel hang up - {err}"))?;

    let mut buffer = Vec::new();
    buffer.resize(64, 0u8);

    loop {
        let n = stream.as_ref().read(&mut buffer).map_err(|err| {
            eprintln!("ERROR: could not read stream - {err}");
            let _ = sender.send(ChatMessage::Disconnected(stream.clone()));
        })?;

        sender
            .send(ChatMessage::Message(stream.clone(), buffer[0..n].to_vec()))
            .map_err(|err| eprintln!("ERROR: channel hang up - {err}"))?;
    }
}

fn server(receiver: Receiver<ChatMessage>) {
    let mut clients = HashMap::new();

    for message in receiver.iter() {
        match message {
            ChatMessage::Connected(stream) => {
                let addr = stream
                    .peer_addr()
                    .expect("local address to be here")
                    .clone();
                clients.insert(
                    addr,
                    Client {
                        conn: stream.clone(),
                    },
                );
            }
            ChatMessage::Disconnected(stream) => {
                let addr = stream.peer_addr().expect("local address to be here");
                clients.remove(&addr);
                todo!()
            }
            ChatMessage::Message(stream, bytes) => {
                let owner_addr = stream.peer_addr().expect("local address to be here");

                let message = String::from_utf8(bytes).map_err(|err| {
                    eprintln!("ERROR: could not undertand message - {err}");
                });

                if let Ok(message) = message {
                    for (&addr, client) in clients.iter() {
                        if owner_addr != addr {
                            let _ = write!(client.conn.as_ref(), "{message}");
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let addr = "0.0.0.0:3000";

    let listener = TcpListener::bind(addr)
        .map_err(|err| eprintln!("ERROR: could not open TcpListener at {addr} - {err}"))?;

    println!("Tcp server listening at {addr}");

    let (sender, receiver) = channel::<ChatMessage>();

    thread::spawn(|| server(receiver));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let sender = sender.clone();
                thread::spawn(|| client(sender, stream.into()));
            }
            Err(err) => {
                eprintln!("ERROR: while reading the incoming TcpStream - {err}")
            }
        }
    }

    Ok(())
}
