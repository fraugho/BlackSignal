use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct InitMessage {
    pub user_id: String,
    pub ws_id: String,
    pub username: String,
    pub user_map: HashMap<String, String>,
}

impl InitMessage {
    pub fn new(user_id: String, ws_id: String, username: String, user_map: HashMap<String, String>) -> Self {
        InitMessage {
            user_id,
            ws_id,
            username,
            user_map,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserInfo{
    pub user_id: String,
    pub ws_id: String,
    pub username: String,
}

impl UserInfo {
    pub fn new(user_id: String, ws_id: String, username: String) -> Self {
        UserInfo {
            user_id,
            ws_id,
            username,
        }
    }
}

// Enum for different types of messages
#[derive(Serialize, Deserialize, Clone)]
pub enum UserMessage {
    Basic(BasicMessage),
    Image(ImageMessage),
    Notification(NotificationMessage),
    Typing(TypingMessage),
    UserRemoval(UserRemovalMessage),
    UserAddition(UserAdditionMessage),
    NewUser(NewUserMessage),
    ChangeRoom(ChangeRoomMessage),
    UsernameChange(UsernameChangeMessage),
    CreateRoomChange(CreateRoomChangeMessage),
    Initialization(InitMessage),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserAdditionMessage{
    pub user_id: String,
    pub username: String,
}

impl UserAdditionMessage {
    pub fn new(user_id: String, username: String) -> Self {
        UserAdditionMessage {
            user_id,
            username,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewUserMessage{
    pub user_id: String,
    pub username: String,
}

impl NewUserMessage {
    pub fn new(user_id: String, username: String) -> Self {
        NewUserMessage {
            user_id,
            username,
        }
    }
}

// Basic Message
#[derive(Serialize, Deserialize, Clone)]
pub struct BasicMessage {
    pub content: String,
    pub sender_id: String,
    pub timestamp: u64,
    pub message_id: String,
    pub room_id: String,
    pub ws_id: String,
}

// Image Message
#[derive(Serialize, Deserialize, Clone)]
pub struct ImageMessage {
    pub image_url: String,
    pub sender_id: String,
}

// ... implement MessageTrait for ImageMessage and other message types

// Notification Message
#[derive(Serialize, Deserialize, Clone)]
pub struct NotificationMessage {
    pub sender_id: String,
}

// Typing Message
#[derive(Serialize, Deserialize, Clone)]
pub struct TypingMessage {
    pub sender_id: String,
}

// User Removal Message
#[derive(Serialize, Deserialize, Clone)]
pub struct UserRemovalMessage {
    pub removed_user: String,
    pub room_id: String,
    pub sender_id: String,
}

// Change Room Message
#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeRoomMessage {
    pub room_id: String,
    pub sender_id: String,
}

// Username Change Message
#[derive(Serialize, Deserialize, Clone)]
pub struct UsernameChangeMessage {
    pub new_username: String,
    pub sender_id: String,
}

impl UsernameChangeMessage {
    pub fn new(sender_id: String, new_username: String) -> Self {
        UsernameChangeMessage {
            sender_id,
            new_username,
        }
    }
}

// Create Room Change Message
#[derive(Serialize, Deserialize, Clone)]
pub struct CreateRoomChangeMessage {
    pub room_name: String,
    pub sender_id: String,
}


/*
#[derive(Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub message_id: String,
    pub content: String,
    pub ws_id: String,
    pub sender_id: String,
    pub room_id: String,
    pub timestamp: u64,
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

//new hypotheticl messages

#[derive(Serialize, Deserialize, Clone)]
pub struct BasicMessage {
    pub content: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageMessageContent {
    pub image_url: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NotificationMessage {
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TypingMessage {
    pub sender_id: String,
    pub message_type: MessageTypes,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct UserRemovalMessage {
    pub removed_user: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeRoomMessage {
    pub room_id: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UsernameChangeMessage {
    pub new_username: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateRoomChangeMessage {
    pub room_name: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IncomingMessage {
    pub content: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}
*/

