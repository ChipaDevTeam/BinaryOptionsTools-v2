use uuid::Uuid;

pub mod keep_alive;
pub mod profile;

#[derive(Debug)]
pub struct Command<T> {
    id: Uuid,
    data: T,
}

impl<T> Command<T> {
    pub fn new(data: T) -> (Uuid, Self) {
        let id = Uuid::new_v4();
        (id, Command { id, data })
    }

    pub fn from_id(id: Uuid, data: T) -> Self {
        Command { id, data }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}
