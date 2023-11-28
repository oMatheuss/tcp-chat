use std::net::{TcpListener, TcpStream};
use std::result;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::Arc;

type Result<T> = result::Result<T, ()>;

enum ChatMessage {
	Connected(Arc<TcpStream>),
	Disconnected,
	Message(Vec<u8>)
}

struct Client {
	sender: Sender<ChatMessage>,
    stream: Arc<TcpStream>
}

impl Client {
	fn handle_connection(self) -> Result<()> {
		self.sender
			.send(ChatMessage::Connected(self.stream.clone()))
			.map_err(|err| eprintln!("ERROR: channel hang up - {err}"))?;

		let mut buffer = Vec::new();
		buffer.resize(64, 0u8);

		loop {
			let n = self.stream.as_ref().read(&mut buffer).map_err(|err| {
				eprintln!("ERROR: could not read stream - {err}");
				let _ = self.sender.send(ChatMessage::Disconnected);
			})?;

			self.sender
				.send(ChatMessage::Message(buffer[0..n].to_vec()))
				.map_err(|err| eprintln!("ERROR: channel hang up - {err}"))?;
		}
	}
}


struct Server {
    receiver: Receiver<ChatMessage>,
	clients: Vec<Arc<TcpStream>>
}

impl Server {
    fn new(recv: Receiver<ChatMessage>) -> Self {
        Self {
            receiver: recv,
			clients: Vec::new()
        }
    }

    fn start(&mut self) {
        for message in self.receiver.iter() {
            match message {
                ChatMessage::Connected(stream) => {
                    self.clients.push(stream);
                }
                ChatMessage::Disconnected => {
                    todo!()
                }
                ChatMessage::Message(message) => {
					let message = String::from_utf8(message);

					match message {
						Ok(message) => {
							println!("{message}");
							//let _ = write!(self.clients[0].as_ref(), "{message}");
						},
						Err(error) => eprintln!("{}", error),
					}

					
				},
            }
        }
    }
}

fn main() -> Result<()> {
    let addr = "127.0.0.1:3000";

    let listener = TcpListener::bind(addr)
        .map_err(|err| eprintln!("ERROR: could not open TcpListener at {addr} - {err}"))?;

    println!("Tcp server listening at {addr}");

    let (sender, receiver) = channel::<ChatMessage>();

    thread::spawn(|| {
        let mut server = Server::new(receiver);
        server.start();
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let sender = sender.clone();

                thread::spawn(|| {
					let client = Client { stream: stream.into(), sender };
                    let _ = client.handle_connection();
                });
            }
            Err(err) => {
                eprintln!("ERROR: while reading the incoming TcpStream - {err}")
            }
        }
    }

    Ok(())
}
