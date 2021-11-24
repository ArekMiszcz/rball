use laminar::{Packet};
use crossbeam_channel::{Sender};

use shared::message::{Message};
use super::world::{World};

pub trait IObserver {
    fn get_id(&self) -> i32;
    fn update(&mut self, world: &mut World, sender: &Sender<Packet>, msg: &Message, ip: &String);
}

pub trait ISubject<'a, T: IObserver> {
    fn attach(&mut self, observer: &'a mut T);
    fn detach(&mut self, observer: &'a T);
    fn notify_observers(&mut self, world: &mut World, sender: &Sender<Packet>, msg: &Message, ip: &String);
}

pub struct Subject<'a, T: IObserver> {
    observers: Vec<&'a mut T>,
}

impl<'a, T: IObserver> ISubject<'a, T> for Subject<'a, T> {
    fn attach(&mut self, observer: &'a mut T) {
        self.observers.push(observer);
    }
    fn detach(&mut self, observer: &'a T) {
        if let Some(idx) = self.observers.iter().position(|x| x.get_id() == observer.get_id()) {
            self.observers.remove(idx);
        }
    }
    fn notify_observers(&mut self, world: &mut World, sender: &Sender<Packet>, msg: &Message, ip: &String) {
        for observer in self.observers.iter_mut() {
            observer.update(world, sender, msg, ip);
        }
    }
}