use uuid::Uuid;

pub mod profile;
pub mod keep_alive;

#[derive(Debug)]
pub struct Command<T> {
    id: Uuid,
    data: T
}


impl<T> Command<T> {
    pub fn new(data: T) -> (Uuid, Self) {
        let id = Uuid::new_v4();
        (id, Command { id, data })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}