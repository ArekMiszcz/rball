use std::collections::HashMap;
use std::thread::JoinHandle;
use std::{thread};
use std::str::FromStr;
use std::time::{Duration, Instant};

use log::{info, trace, error};

use rapier2d::prelude::*;

use crossbeam_channel::{unbounded, Receiver, Sender, SendError};

use laminar::{Packet, Socket, SocketEvent};
use shared::message::{Position, Behavior, Message, MessageKind};

use serde::{Serialize};
use serde_json::{Value, json};

#[derive(Clone)]
struct PhysicsEngine {
    channels: HashMap<String, (Sender<NetworkCommand>, Receiver<NetworkCommand>)>,
    handles: HashMap<String, RigidBodyHandle>,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: JointSet,
    islands: IslandManager
}

impl PhysicsEngine {
    fn initialize(&mut self) {
        /*
         * Ground.
         */
        let ground_size = 5.0;
        let ground_area = vector![970.0, 580.0];

        /*
         * Frame.
         */
        let offset_x = 27.0;
        let offset_y = -10.0;

        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![offset_x, ground_area.y / 2.0 - offset_y])
            .build();
        let collider = ColliderBuilder::cuboid(ground_size, ground_area.y / 2.0).build();
        self.insert_body(String::from("left"), rigid_body, collider);

        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![ground_area.x + offset_x, ground_area.y / 2.0 - offset_y])
            .build();
        let collider = ColliderBuilder::cuboid(ground_size, ground_area.y / 2.0).build();
        self.insert_body(String::from("right"), rigid_body, collider);

        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![ground_area.x / 2.0 + offset_x, 0.0 - offset_y])
            .build();
        let collider = ColliderBuilder::cuboid(ground_area.x / 2.0, ground_size).build();
        self.insert_body(String::from("top"), rigid_body, collider);

        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![ground_area.x / 2.0 + offset_x, ground_area.y - offset_y])
            .build();
        let collider = ColliderBuilder::cuboid(ground_area.x / 2.0, ground_size).build();
        self.insert_body(String::from("bottom"), rigid_body, collider);

        // Build ball
        let rad = 8.0;
        let ball_body = RigidBodyBuilder::new_dynamic()
            .translation(vector![250.0, 250.0])
            .linear_damping(0.5)
            .angular_damping(1.0)
            .build();
        let collider = ColliderBuilder::ball(rad)
            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
            .restitution(0.7)
            .active_collision_types(ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC)
            .build();
        self.insert_body(String::from("ball"), ball_body, collider);
    }

    fn insert_body(
        &mut self,
        name: String,
        body: RigidBody,
        collider: Collider,
    ) -> RigidBodyHandle {
        let rigid_body_set = &mut self.bodies;

        let handle = rigid_body_set.insert(body);
        self.colliders.insert_with_parent(collider, handle, rigid_body_set);
        self.handles.insert(name, handle);

        handle
    }

    fn remove_body(&mut self, name: &str) -> Result<(), &'static str> {
        let rigid_body_set = &mut self.bodies;

        if let Some(rigid_body_handle) = self.handles.remove(name) {
            rigid_body_set.remove(
                rigid_body_handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.joints,
            );

            return Ok(());
        }

        Err("Body not found")
    }

    fn start_simulation(&mut self) {
        self.initialize();

        let gravity = vector![0.0, 0.0];
        let integration_parameters = IntegrationParameters::default();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut ccd_solver = CCDSolver::new();
        let physics_hooks = ();
        let event_handler = ();

        let mut physics_pipeline = PhysicsPipeline::new();

        let delay = Duration::from_millis(1);

        loop {
            self.handle_command(&narrow_phase);

            physics_pipeline.step(
                &gravity,
                &integration_parameters,
                &mut self.islands,
                &mut broad_phase,
                &mut narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints,
                &mut ccd_solver,
                &physics_hooks,
                &event_handler,
            );

            self.send_telemetrics();

            thread::sleep(delay);
        }
    }

    fn handle_command(&mut self, narrow_phase: &NarrowPhase) {
        #[derive(Debug)]
        struct BallCollision {
            direction: Vector<f32>,
            rigid_body_handle: RigidBodyHandle,
        }

        fn handle_ball_collision(narrow_phase: &NarrowPhase, body_set: &RigidBodySet, handle_set: &HashMap<String, RigidBodyHandle>, collider_set: &ColliderSet, rigid_body_handle: &RigidBodyHandle) -> Option<BallCollision> {
            let ball_rigid_body_handle = handle_set.get("ball").unwrap();
            let ball_rigid_body = body_set.get(*ball_rigid_body_handle).unwrap();

            for ball_collider_handle in ball_rigid_body.colliders().iter() {
                let player_rigid_body = body_set.get(*rigid_body_handle).unwrap();
                for player_collider_handle in player_rigid_body.colliders().iter() {
                    if narrow_phase.intersection_pair(*ball_collider_handle, *player_collider_handle) == Some(true) {
                        let ball_collider = collider_set.get(*ball_collider_handle).unwrap();
                        let direction = player_rigid_body.translation() - ball_collider.translation();

                        return Some(BallCollision {
                            direction,
                            rigid_body_handle: *ball_rigid_body_handle
                        });
                    }
                }
            }

            return None
        }


        let (sender, _) = &self.channels.get("network").unwrap().clone();
        let (_, receiver) = &self.channels.get("physics").unwrap();
        match receiver.try_recv() {
            Ok(command) => {
                match command.kind {
                    CommandKind::AddPlayer => {
                        let data = json!(command.data);
                        let name = String::from(data["name"].as_str().unwrap());
                        let nickname = String::from(data["nickname"].as_str().unwrap());
                        
                        let rad = 15.0;
                        let translation = vector![128.0, 301.0];
                        let ball_body = RigidBodyBuilder::new_kinematic_velocity_based()
                            .translation(translation)
                            .build();
                        let collider = ColliderBuilder::ball(rad)
                            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                            .sensor(true)
                            .active_collision_types(ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC)
                            .build();

                        self.insert_body(name.clone(), ball_body, collider);

                        sender.send(NetworkCommand {
                            kind: CommandKind::AddPlayerAck,
                            data: json!({
                                "name": name,
                                "nickname": nickname,
                                "translation": { "x": translation.x, "y": translation.y },
                            })
                        }).unwrap();
                    },
                    CommandKind::ChangePlayerTeam => {
                        let data = json!(command.data);
                        let name = data["name"].as_str().unwrap();
                        let team = data["team"].as_str().unwrap();

                        sender.send(NetworkCommand {
                            kind: CommandKind::ChangePlayerTeamAck,
                            data: json!({
                                "name": name,
                                "team": team,
                            })
                        }).unwrap();
                    },
                    CommandKind::MovePlayer | CommandKind::MoveEnemy => {
                        let data = json!(&command.data);
                        let name = data.get("name").unwrap();
                        let velocity = data.get("velocity").unwrap().as_object().unwrap();

                        if let Some(rigid_body_handle) = self.handles.get(name.as_str().unwrap()) {
                            let (x, y) = (velocity["x"].as_f64().unwrap(), velocity["y"].as_f64().unwrap());
                            let linvel = vector![x as f32, y as f32];
    
                            let rigid_body = self.bodies.get_mut(*rigid_body_handle).unwrap();
                            rigid_body.set_linvel(linvel, true);
                            
                            if let Some(ball_collision) = handle_ball_collision(narrow_phase, &self.bodies, &self.handles, &self.colliders, &rigid_body_handle) {
                                let power = 100.0;
                                let ball_rigid_body = self.bodies.get_mut(ball_collision.rigid_body_handle).unwrap();
                                ball_rigid_body.apply_impulse(-ball_collision.direction * power, true);
                            }
                        }
                    },
                    CommandKind::KickBall => {
                        let data = json!(&command.data);
                        let name = data.get("name").unwrap();

                        if let Some(rigid_body_handle) = self.handles.get(name.as_str().unwrap()) {
                            let player_rigid_body = self.bodies.get(*rigid_body_handle).unwrap();
                            let player_translation = player_rigid_body.translation().clone();

                            let ball_rigid_body_handle = self.handles.get("ball").unwrap();
                            let ball_rigid_body = self.bodies.get_mut(*ball_rigid_body_handle).unwrap();
                            let ball_translation = ball_rigid_body.translation();

                            let power = 500.0;
                            let direction = player_translation - ball_translation;
                            let distance = ((ball_translation.x - player_translation.x).powi(2) + (ball_translation.y - player_translation.y).powi(2)).sqrt();
                            
                            if distance < 30.0 {
                                ball_rigid_body.apply_impulse(-direction * power, true);
                            }
                        }
                    },
                    CommandKind::DisconnectPlayer => {
                        let data = json!(&command.data);
                        let name = data.get("name").unwrap().as_str().unwrap();

                        if let Ok(_) = self.remove_body(name) {
                            sender.send(NetworkCommand {
                                kind: CommandKind::DisconnectPlayerAck,
                                data: json!({
                                    "name": name
                                })
                            }).unwrap();
                        }
                    },
                    _ => trace!("Unknown command: {:?}", command.kind)
                }
            },
            Err(e) => {
                if !e.is_empty() {
                    error!("Something went wrong when receiving, error: {:?}", e)
                }
            }
        }
    }

    fn send_telemetrics(&mut self) {
        let (sender, _) = &self.channels.get("network").unwrap();
        let mut data: HashMap<String, Value> = HashMap::new();

        for (name, handle) in self.handles.iter() {
            let rigid_body = self.bodies[*handle].clone();
            let translation = rigid_body.translation();

            data.insert(String::from(name), json!({
                "translation": { "x": translation.x, "y": translation.y },
            }));
        }

        let telemetrics = json!(data);
        let command_telemetrics = NetworkCommand {
            kind: CommandKind::Telemetrics,
            data: telemetrics
        };

        sender.send(command_telemetrics).unwrap();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
enum TeamKind {
    RedTeam,
    SpecTeam,
    BlueTeam
}

impl FromStr for TeamKind {
    type Err = ();
    fn from_str(input: &str) -> Result<TeamKind, Self::Err> {
        match input {
            "RedTeam"  => Ok(TeamKind::RedTeam),
            "SpecTeam"  => Ok(TeamKind::SpecTeam),
            "BlueTeam"  => Ok(TeamKind::BlueTeam),
            _      => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
struct Client {
    ip_address: String,
    nickname: String,
    team: TeamKind
}

impl Client {
    fn assign_team(&mut self, team: TeamKind) {
        self.team = team;
    }
}

#[derive(Debug)]
enum CommandKind {
    AddPlayer,
    AddPlayerAck,

    ChangePlayerTeam,
    ChangePlayerTeamAck,

    DisconnectPlayer,
    DisconnectPlayerAck,

    MovePlayer,
    MoveEnemy,

    KickBall,

    Telemetrics,
}

#[derive(Debug)]
struct NetworkCommand {
    kind: CommandKind,
    data: Value,
}

#[derive(Debug, Clone)]
struct Network {
    channels: HashMap<String, (Sender<NetworkCommand>, Receiver<NetworkCommand>)>,
    clients: Vec<Client>,
}

impl Network {
    fn add_client(&mut self, client: Client) {
        self.clients.push(client);
    }

    fn get_clients(&mut self) -> &mut Vec<Client> {
        &mut self.clients
    }

    fn get_client(&mut self, ip_address: &String) -> Option<&mut Client> {
        self.clients.iter_mut().find(|client| client.ip_address.eq(ip_address))
    }

    fn delete_client(&mut self, client: &Client) {
        let index = self.clients.iter().position(|c| c.ip_address.eq(&client.ip_address)).unwrap();
        self.clients.remove(index);
    }

    fn send_command(&mut self, command: NetworkCommand) -> Result<(), SendError<NetworkCommand>> {
        let (sender, _) = &self.channels.get("physics").unwrap();
        sender.send(command)
    }

    fn handle_message(&mut self, msg: &Message, ip_address: &String) {
        match msg.kind {
            MessageKind::Connect => {
                info!(
                    "Received connect message: {:?} from ip: {:?}",
                    msg, ip_address
                );

                let data: Value = serde_json::from_str(&msg.payload).unwrap();
                let nickname = data["nickname"].as_str().unwrap();

                self.add_client(Client {
                    ip_address: ip_address.clone(),
                    nickname: String::from(nickname),
                    team: TeamKind::SpecTeam
                });

                self.send_command(NetworkCommand {
                    kind: CommandKind::AddPlayer,
                    data: json!({
                        "name": ip_address.clone(),
                        "nickname": String::from(nickname)
                    }),
                }).unwrap();
            }
            MessageKind::Data => {
                let clients = self.get_clients().clone();
                for client in clients.iter() {
                    if client.ip_address.eq(ip_address) {
                        let payload: Value = serde_json::from_str(&msg.payload).unwrap();
                        let action = payload["action"].as_str().unwrap();

                        match action {
                            "CHANGE_PLAYER_TEAM" => {
                                if let Ok(team) = TeamKind::from_str(payload["team"].as_str().unwrap()) {
                                    if let Some(client) = self.get_client(ip_address) {
                                        client.assign_team(team.clone());

                                        self.send_command(NetworkCommand {
                                            kind: CommandKind::ChangePlayerTeam,
                                            data: json!({ 
                                                "name": ip_address.clone(),
                                                "team": team
                                            })
                                        }).unwrap();
                                    }
                                }
                            },
                            "PLAYER_MOVED" => {
                                if let Some(position) = payload["position"].as_object() {
                                    let data = json!({
                                        "name": ip_address.clone(),
                                        "velocity": position
                                    });
    
                                    self.send_command(NetworkCommand {
                                        kind: CommandKind::MovePlayer,
                                        data,
                                    }).unwrap();
                                }
                            },
                            "PLAYER_KICKED" => {
                                let data = json!({
                                    "name": ip_address.clone()
                                });

                                self.send_command(NetworkCommand {
                                    kind: CommandKind::KickBall,
                                    data,
                                }).unwrap();
                            },
                            "PLAYER_DISCONNECTED" => {
                                self.delete_client(client);

                                let data = json!({
                                    "name": ip_address.clone()
                                });

                                self.send_command(NetworkCommand {
                                    kind: CommandKind::DisconnectPlayer,
                                    data,
                                }).unwrap();
                            },
                            _ => trace!("Unknown action: {:?}", payload)
                        }
                    } else if self.get_client(ip_address).is_some() {
                        let behavior: Behavior = serde_json::from_str(&msg.payload).unwrap();
                        match behavior.action.as_str() {
                            "ENEMY_MOVED" => {
                                if let Some(position) = behavior.position {
                                    let data = json!({
                                        "name": ip_address.clone(),
                                        "velocity": position
                                    });
    
                                    self.send_command(NetworkCommand {
                                        kind: CommandKind::MoveEnemy,
                                        data,
                                    }).unwrap();
                                }
                            },
                            _ => trace!("Unknown action: {:?}", behavior)
                        }
                    }
                }
                
            }
            MessageKind::Timeout => {
                // println!(
                //     "Received timeout message: {:?} from ip: {:?}",
                //     msg, ip_address
                // );
            }
        }
    }

    fn handle_socket_event(&mut self, event_receiver: &Receiver<SocketEvent>) {
        match event_receiver.try_recv() {
            Ok(socket_event) => match socket_event {
                SocketEvent::Packet(packet) => {
                    let payload = String::from_utf8_lossy(packet.payload());
                    let message: Message = serde_json::from_str(&payload).unwrap();
                    let ip_address = packet.addr().to_string();

                    self.handle_message(&message, &ip_address);
                }
                SocketEvent::Timeout(ip_address) => {
                    let message = Message {
                        kind: MessageKind::Timeout,
                        payload: String::new(),
                    };

                    self.handle_message(&message, &ip_address.to_string());
                }
                _ => (),
            },
            Err(e) => {
                if !e.is_empty() {
                    error!("Something went wrong when receiving, error: {:?}", e)
                }
            }
        }
    }

    fn handle_telemetrics(&mut self, packet_sender: &Sender<Packet>, last_position: &mut HashMap<String, HashMap<String, Vector<f32>>>, elapsed: Duration) {
        let (_, receiver) = &self.channels.get("network").unwrap();
        let response_duration = 30;

        match receiver.try_recv() {
            Ok(command) => {
                match command.kind {
                    CommandKind::AddPlayerAck => {
                        let player_data = json!(command.data);

                        let (x, y) = (player_data["translation"]["x"].as_f64().unwrap() as f32, player_data["translation"]["y"].as_f64().unwrap() as f32);
                        let new_player_name = String::from(player_data["name"].as_str().unwrap());
                        let new_player_nickname = String::from(player_data["nickname"].as_str().unwrap());

                        let clients = self.get_clients().clone();

                        for client in &clients {
                            let add_player_ack_message = Message {
                                kind: MessageKind::Data,
                                payload: json!({
                                    "action": String::from("PLAYER_ADD_ACK"),
                                    "name": new_player_name,
                                    "nickname": new_player_nickname,
                                    "position": { "x": x, "y": y }
                                }).to_string(),
                            };

                            let data_message = serde_json::to_string(&add_player_ack_message).unwrap().into_bytes();
                            let unreliable = Packet::unreliable(client.ip_address.parse().unwrap(), data_message.clone());
                            packet_sender.send(unreliable).unwrap();
                        }

                        for client in &clients {
                            if client.ip_address.ne(&new_player_name) {
                                let last_client_position = last_position.get_mut(&client.ip_address).unwrap();

                                if let Some(last_player_position) = last_client_position.get("player_position") {
                                    let existing_player_message = Message {
                                        kind: MessageKind::Data,
                                        payload: json!({
                                            "action": String::from("PLAYER_ADD_ACK"),
                                            "name": client.ip_address.clone(),
                                            "nickname": client.nickname,
                                            "position": { "x": last_player_position.x, "y": last_player_position.y },
                                            "team": client.team
                                        }).to_string(),
                                    };
    
                                    let data_message = serde_json::to_string(&existing_player_message).unwrap().into_bytes();
                                    let unreliable = Packet::unreliable(new_player_name.parse().unwrap(), data_message.clone());
                                    packet_sender.send(unreliable).unwrap();
                                }
                            }
                        }
                    },
                    CommandKind::ChangePlayerTeamAck => {
                        let player_data = json!(command.data);
                        let player_name = player_data["name"].as_str().unwrap();
                        let team = player_data["team"].as_str().unwrap();

                        let clients = self.get_clients().clone();

                        for client in &clients {
                            let add_player_ack_message = Message {
                                kind: MessageKind::Data,
                                payload: json!({
                                    "action": String::from("CHANGE_PLAYER_TEAM_ACK"),
                                    "name": player_name,
                                    "team": team,
                                }).to_string(),
                            };

                            let data_message = serde_json::to_string(&add_player_ack_message).unwrap().into_bytes();
                            let unreliable = Packet::unreliable(client.ip_address.parse().unwrap(), data_message.clone());
                            packet_sender.send(unreliable).unwrap();
                        }

                        for client in &clients {
                            if client.ip_address.ne(&player_name) {
                                let existing_player_message = Message {
                                    kind: MessageKind::Data,
                                    payload: json!({
                                        "action": String::from("CHANGE_PLAYER_TEAM_ACK"),
                                        "name": client.ip_address.clone(),
                                        "team": client.team
                                    }).to_string(),
                                };

                                let data_message = serde_json::to_string(&existing_player_message).unwrap().into_bytes();
                                let unreliable = Packet::unreliable(player_name.parse().unwrap(), data_message.clone());
                                packet_sender.send(unreliable).unwrap();
                            }
                        }
                    },
                    CommandKind::Telemetrics => {
                        fn handle_ball(packet_sender: &Sender<Packet>, client_name: String, telemetrics: &Value, last_position: &mut HashMap<String, HashMap<String, Vector<f32>>>) {
                            let ball_telemetrics = &telemetrics["ball"];
                            let (x, y) = (ball_telemetrics["translation"]["x"].as_f64().unwrap() as f32, ball_telemetrics["translation"]["y"].as_f64().unwrap() as f32);
                            let last_client_position = last_position.get_mut(&client_name).unwrap();

                            let position_message = Message {
                                kind: MessageKind::Data,
                                payload: serde_json::to_string(&Behavior {
                                    action: String::from("BALL_MOVED"),
                                    position: Some(Position { x, y })
                                }).unwrap()
                            };

                            let data_message = serde_json::to_string(&position_message).unwrap().into_bytes();
                            let unreliable = Packet::unreliable(client_name.parse().unwrap(), data_message.clone());

                            if let Some(last_ball_position) = last_client_position.get_mut("ball_position") {
                                if last_ball_position.x != x || last_ball_position.y != y {
                                    packet_sender.send(unreliable).unwrap();
                                    *last_ball_position = vector![x, y];
                                }
                            } else {
                                packet_sender.send(unreliable).unwrap();
                                last_client_position.insert(String::from("ball_position"), vector![x, y]);
                            }
                        }

                        fn handle_player(packet_sender: &Sender<Packet>, client_name: String, telemetrics: &Value, last_position: &mut HashMap<String, HashMap<String, Vector<f32>>>) {
                            let player_telemetrics = &telemetrics[client_name.clone()];

                            if !player_telemetrics.is_null() {
                                let (x, y) = (player_telemetrics["translation"]["x"].as_f64().unwrap().round() as f32, player_telemetrics["translation"]["y"].as_f64().unwrap().round() as f32);
                                let last_client_position = last_position.get_mut(&client_name).unwrap();

                                let position_message = Message {
                                    kind: MessageKind::Data,
                                    payload: serde_json::to_string(&Behavior {
                                        action: String::from("PLAYER_MOVED"),
                                        position: Some(Position { x, y })
                                    }).unwrap()
                                };
    
                                let data_message = serde_json::to_string(&position_message).unwrap().into_bytes();
                                let unreliable = Packet::unreliable(client_name.parse().unwrap(), data_message.clone());

                                if let Some(last_player_position) = last_client_position.get_mut("player_position") {
                                    if last_player_position.x != x || last_player_position.y != y {
                                        packet_sender.send(unreliable).unwrap();
                                        *last_player_position = vector![x, y];
                                    }
                                } else {
                                    packet_sender.send(unreliable).unwrap();
                                    last_client_position.insert(String::from("player_position"), vector![x, y]);
                                }
                            }
                        }

                        fn handle_enemy(packet_sender: &Sender<Packet>, client_name: String, enemy_name: String, telemetrics: &Value, last_position: &mut HashMap<String, HashMap<String, Vector<f32>>>) {
                            let player_telemetrics = &telemetrics[enemy_name.clone()];
                            if !player_telemetrics.is_null() {
                                if let Some(last_client_position) = last_position.get_mut(&client_name) {
                                    let (x, y) = (player_telemetrics["translation"]["x"].as_f64().unwrap().round() as f32, player_telemetrics["translation"]["y"].as_f64().unwrap().round() as f32);

                                    let position_message = Message {
                                        kind: MessageKind::Data,
                                        payload: json!({
                                            "action": String::from("ENEMY_MOVED"),
                                            "name": enemy_name,
                                            "position": { "x": x, "y": y }
                                        }).to_string()
                                    };
        
                                    let data_message = serde_json::to_string(&position_message).unwrap().into_bytes();
                                    let unreliable = Packet::unreliable(client_name.parse().unwrap(), data_message.clone());

                                    if let Some(last_enemy_position) = last_client_position.get_mut(enemy_name.as_str()) {
                                        if last_enemy_position.x != x || last_enemy_position.y != y {
                                            packet_sender.send(unreliable).unwrap();
                                            *last_enemy_position = vector![x, y];
                                        }
                                    } else {
                                        packet_sender.send(unreliable).unwrap();
                                        last_client_position.insert(enemy_name, vector![x, y]);
                                    }
                                }
                            }
                        }

                        let clients = self.get_clients().clone();

                        for client in &clients {
                            if last_position.get(&client.ip_address).is_none() {
                                last_position.insert(client.ip_address.clone(), HashMap::new());
                            }

                            let telemetrics = json!(command.data);

                            if elapsed.as_millis() % response_duration == 0 {
                                handle_ball(packet_sender, client.ip_address.clone(), &telemetrics, last_position);
                                handle_player(packet_sender, client.ip_address.clone(), &telemetrics, last_position);
                            }

                            for enemy in &clients {
                                if enemy.ip_address.eq(&client.ip_address) {
                                    continue;
                                }

                                if elapsed.as_millis() % response_duration == 0 {
                                    handle_enemy(packet_sender, client.ip_address.clone(), enemy.ip_address.clone(), &telemetrics, last_position);
                                }
                            }
                        }
                    },
                    CommandKind::DisconnectPlayerAck => {
                        let player_data = json!(command.data);
                        let player_name = player_data["name"].as_str().unwrap();
                        let disconnect_player_ack_message = Message {
                            kind: MessageKind::Data,
                            payload: json!({
                                "action": String::from("PLAYER_DISCONNECT_ACK"),
                                "name": String::from(player_name)
                            }).to_string(),
                        };

                        let clients = self.get_clients().clone();

                        for client in &clients {
                            let data_message = serde_json::to_string(&disconnect_player_ack_message).unwrap().into_bytes();
                            let unreliable = Packet::unreliable(client.ip_address.parse().unwrap(), data_message.clone());
                            packet_sender.send(unreliable).unwrap();
                        }

                        last_position.remove(player_name);
                    },
                    _ => trace!("Unknown command: {:?}", command.kind)
                }
            },
            Err(e) => {
                if !e.is_empty() {
                    error!("Something went wrong when receiving, error: {:?}", e)
                }
            }
        }
    }

    fn start_server(&mut self, server_ip_address: &str) {
        match Socket::bind(server_ip_address) {
            Ok(mut socket) => {
                let (packet_sender, event_receiver) =
                    (socket.get_packet_sender(), socket.get_event_receiver());

                thread::spawn(move || socket.start_polling());

                info!("Waiting for connection at: {:?}", server_ip_address);

                let delay = Duration::from_nanos(1);
                let mut start = Instant::now();
                let mut position_state: HashMap<String, HashMap<String, Vector<f32>>> = HashMap::new();

                loop {
                    let duration = start.elapsed();

                    self.handle_socket_event(&event_receiver);
                    self.handle_telemetrics(&packet_sender, &mut position_state, duration);

                    if duration.as_millis() > 120 {
                        start = Instant::now();
                    }

                    thread::sleep(delay);
                }
            }
            Err(e) => error!("Something went wrong: {:?}", e),
        }
    }
}

#[derive(Clone)]
struct State {}

impl State {
    // fn add_client(&mut self, client: Client) {
    //     self.clients.push(client);
    // }

    // fn get_clients(&self) -> Vec<Client> {
    //     self.clients.clone()
    // }
}

struct World {}

impl World {
    fn setup_physics_engine(
        &self,
        channels: HashMap<String, (Sender<NetworkCommand>, Receiver<NetworkCommand>)>,
    ) -> JoinHandle<()> {
        let mut physics_engine = PhysicsEngine {
            channels,
            handles: HashMap::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: JointSet::new(),
            islands: IslandManager::new(),
        };

        thread::spawn(move || physics_engine.start_simulation())
    }

    fn setup_network(
        &self,
        channels: HashMap<String, (Sender<NetworkCommand>, Receiver<NetworkCommand>)>,
    ) -> JoinHandle<()> {
        let mut network = Network { channels, clients: Vec::new() };
        thread::spawn(move || network.start_server("127.0.0.1:12350"))
    }

    fn run(self) {
        let mut channels: HashMap<String, (Sender<NetworkCommand>, Receiver<NetworkCommand>)> = HashMap::new();

        channels.insert(String::from("network"), unbounded());
        channels.insert(String::from("physics"), unbounded());

        let handles = vec![
            self.setup_physics_engine(channels.clone()),
            self.setup_network(channels.clone()),
        ];

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

fn main() { 
    env_logger::init();

    let world = World {};
    world.run();
}
