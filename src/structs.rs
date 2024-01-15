use serde::{Serialize, Deserialize};

use std::fmt;

use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserData {
    pub uuid: String,
    pub login_username: String,
    pub username: String,
    pub hashed_password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub unique_id: String,
    pub content: String,
    pub sender_id: String,
    pub room_id: String,
    pub timestamp: u64,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UsernameChangedMessage {
    pub old_username: String,
    pub new_username: String,
}

#[derive(Serialize, Deserialize)]
pub struct IncomingMessage {
    pub content: String,
    pub username: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum MessageTypes {
    SetUsername,
    AddToRoom,
    CreateRoom,
    ChangeRoom,
    RemoveFromRoom,
    Basic,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Subscriber {
    pub user_id: String,
    pub room_id: String,
    pub connection_state: SubscriberState,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SubscriberState {
    Active,
    Inactive,
    Offline,
}

impl fmt::Display for SubscriberState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SubscriberState::Active => write!(f, "Active"),
            SubscriberState::Inactive => write!(f, "Inactive"),
            SubscriberState::Offline => write!(f, "Offline"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub name: String,
    pub room_id: String,
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
    pub uuid: String,
    pub username: String,
}