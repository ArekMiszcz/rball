
use shared::client::{Client};

pub trait IState {
    fn get_clients(&mut self) -> Vec<Client>;
    fn add_client(&mut self, client: Client) -> ();
    fn remove_client(&mut self, ip_address: String) -> ();
}

pub struct ServerState {
    pub clients: Vec<Client>,
}

impl IState for ServerState {
    fn get_clients(&mut self) -> Vec<Client> {
        self.clients.clone()
    }
    fn add_client(&mut self, client: Client) -> () {
        self.clients.push(client);
    }
    fn remove_client(&mut self, ip_address: String) -> () {
        if let Some(idx) = self.clients.iter().position(|client| client.ip_address.eq(&ip_address)) {
            self.clients.remove(idx);
        }
    }
}