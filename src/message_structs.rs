use serde::{Serialize, Deserialize};

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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct NotificationMessage {
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize)]
pub struct TypingMessage {
    pub sender_id: String,
    pub message_type: MessageTypes,
}


#[derive(Serialize, Deserialize)]
pub struct UserRemovalMessage {
    pub removed_user: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize)]
pub struct ChangeRoomMessage {
    pub room_id: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize)]
pub struct UsernameChangeMessage {
    pub new_username: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRoomChangeMessage {
    pub room_name: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}

//end

//other proposla still needs to be finished
/*
// Define a trait for common behavior among all messages
pub trait MessageTrait {
    fn get_sender_id(&self) -> &String;
    fn get_message_type(&self) -> MessageType;
}

// Enum for different types of messages
#[derive(Serialize, Deserialize, Clone)]
pub enum Message {
    Basic(BasicMessage),
    Image(ImageMessage),
    Notification(NotificationMessage),
    Typing(TypingMessage),
    UserRemoval(UserRemovalMessage),
    ChangeRoom(ChangeRoomMessage),
    UsernameChange(UsernameChangeMessage),
    CreateRoomChange(CreateRoomChangeMessage),
    // ... add other message types as needed
}

// Enum for message types, consider if this is necessary or if Message enum variants are sufficient
#[derive(Serialize, Deserialize, Clone)]
pub enum MessageType {
    SetUsername,
    AddToRoom,
    CreateRoom,
    ChangeRoom,
    RemoveFromRoom,
    Basic,
    // ... add other message types as needed
}

// Basic Message
#[derive(Serialize, Deserialize)]
pub struct BasicMessage {
    pub content: String,
    pub sender_id: String,
}

impl MessageTrait for BasicMessage {
    fn get_sender_id(&self) -> &String {
        &self.sender_id
    }

    fn get_message_type(&self) -> MessageType {
        MessageType::Basic
    }
}

// Image Message
#[derive(Serialize, Deserialize)]
pub struct ImageMessage {
    pub image_url: String,
    pub sender_id: String,
}

// ... implement MessageTrait for ImageMessage and other message types

// Notification Message
#[derive(Serialize, Deserialize)]
pub struct NotificationMessage {
    pub sender_id: String,
}

// Typing Message
#[derive(Serialize, Deserialize)]
pub struct TypingMessage {
    pub sender_id: String,
}

// User Removal Message
#[derive(Serialize, Deserialize)]
pub struct UserRemovalMessage {
    pub removed_user: String,
    pub sender_id: String,
}

// Change Room Message
#[derive(Serialize, Deserialize)]
pub struct ChangeRoomMessage {
    pub room_id: String,
    pub sender_id: String,
}

// Username Change Message
#[derive(Serialize, Deserialize)]
pub struct UsernameChangeMessage {
    pub new_username: String,
    pub sender_id: String,
}

// Create Room Change Message
#[derive(Serialize, Deserialize)]
pub struct CreateRoomChangeMessage {
    pub room_name: String,
    pub sender_id: String,
}
*/

#[derive(Serialize, Deserialize)]
pub struct IncomingMessage {
    pub content: String,
    pub sender_id: String,
    pub message_type: MessageTypes,
}
