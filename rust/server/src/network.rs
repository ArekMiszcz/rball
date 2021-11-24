use std::thread;
use std::sync::{Arc, Mutex};
use laminar::{SocketEvent, Socket, ErrorKind};

use shared::message::{Message, MessageKind, Behavior};

use super::state::{ServerState};
use super::observer::{IObserver};
use super::world::{IWorld, World};

const SERVER: &str = "127.0.0.1:12350";

pub struct ServerObserver {
    pub id: i32,
    pub state: ServerState
}

pub fn setup_server (world: World, observer: ServerObserver) -> Result<(), ErrorKind> {
    let mut shared_world = Arc::new(Mutex::new(world));
    let observer_world = Arc::new(Mutex::new(observer));
    let mut handles = vec![];

    // Creates the socket
    match Socket::bind(SERVER) {
        Ok(mut socket) => {
            // let mut subject = Subject::new();
            let (packet_sender, event_receiver) = (socket.get_packet_sender(), socket.get_event_receiver());

            thread::spawn(move || socket.start_polling());

            let world = Arc::clone(&mut shared_world);
            let observer = Arc::clone(&observer_world);
            handles.push(thread::spawn(move || {

                println!("Waiting for connection at: {:?}", SERVER);

                let ball_message = Message {
                    kind: MessageKind::Data,
                    payload: serde_json::to_string(&Behavior {
                        action: String::from("BALL_MOVED"),
                        position: None
                    }).unwrap()
                };

                let mut observer = observer.lock().unwrap();
                
                loop {
                    let mut world = world.lock().unwrap();

                    observer.update(&mut world, &packet_sender, &ball_message, &String::new());
                    
                    match event_receiver.recv() {
                        Ok(socket_event) => {
                            match socket_event {
                                SocketEvent::Packet(packet) => {
                                    let payload = String::from_utf8_lossy(packet.payload());
                                    let message: Message = serde_json::from_str(&payload).unwrap();
                                    let ip_address = packet.addr().to_string();

                                    observer.update(&mut world, &packet_sender, &message, &ip_address.to_string());
                                }
                                SocketEvent::Timeout(ip) => {
                                    let message = Message {
                                        kind: MessageKind::Disconnect,
                                        payload: String::new()
                                    };

                                    observer.update(&mut world, &packet_sender, &message, &ip.to_string());
                                }
                                _ => ()
                            }
                        }
                        Err(e) => {
                            println!("Something went wrong when receiving, error: {:?}", e);
                        }
                    }
                }
            }));            
        }
        Err(e) => println!("Something went wrong: {:?}", e)
    }

    let world = Arc::clone(&shared_world);
    handles.push(thread::spawn(move || world.lock().unwrap().start_simulation()));

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}