use serde::{Serialize, Deserialize};

use std::fmt;

use validator::Validate;

use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserData {
    pub user_id: String,
    pub login_username: String,
    pub username: String,
    pub hashed_password: String,
    pub status: ConnectionState,
    pub rooms: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ConnectionState {
    Online,
    Offline,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConnectionState::Online => write!(f, "Offline"),
            ConnectionState::Offline => write!(f, "Offline"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub name: String,
    pub room_id: String,
    pub users: HashSet<String>,
}

#[derive(Deserialize, Validate)]
pub struct LoginForm {
    #[validate(email)]
    pub username: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Deserialize)]
pub struct User {
    pub user_id: String,
    pub username: String,
}


#[derive(Deserialize)]
pub struct RoomUsers {
    pub users: Vec<String>,
}
