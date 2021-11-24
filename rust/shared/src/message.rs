use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageKind {
    Connect,
    Timeout,
    Data
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub kind: MessageKind,
    pub payload: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Behavior {
    pub action: String,
    pub position: Option<Position>
}