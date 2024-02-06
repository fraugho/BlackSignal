use actix::Addr;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use validator::Validate;
use crate::structs::{Room, UserData, LoginForm};
use crate::message_structs::*;
use crate::websocket::{WsActor, WsMessage};

pub type WsActorMap = HashMap<String, Addr<WsActor>>;
pub struct AppState {
    pub db: Arc<Surreal<Client>>,
    pub channels: Arc<Mutex<HashMap<String, Room>>>,
    pub actor_registry: Arc<Mutex<HashMap<String, WsActorMap>>>,
    pub main_room_id: String,
}

impl AppState {
    pub async fn broadcast_message(&self, message: String, room_id: String, user_id: String) {
        let query = "SELECT * FROM rooms WHERE room_id = $room_id;";
        let mut response = match self.db.query(query)
            .bind(("room_id", room_id))
            .await {
                Ok(retrieved) => retrieved,
                Err(e) => {log::error!("Failed to get users in requested room: fn broadcast_message, error: {:?}", e);
                return}
            };
        let rooms: Vec<Room> = match response.take(0) {
            Ok(user) => user,
            Err(e) => {log::error!("Failed to get user data: fn broadcast_message, error: {:?}", e);
            return}
        };
        let actor_registry = self.actor_registry.lock().unwrap();

        for room in rooms {
            if room.users.get(&user_id).is_some() {
                for user in &room.users {
                    if let Some(client) = actor_registry.get(user) {
                        for instance in client.values() {
                            instance.do_send(WsMessage(message.clone()));
                        }
                    }
                }
            } else {
                return;
            }
        }
    }

    pub async fn catch_up(&self, room_id: &str) -> Option<Vec<UserMessage>> {
        let query = "SELECT * FROM messages WHERE room_id = $room_id ORDER BY timestamp ASC;";
        let mut response = match self.db.query(query).bind(("room_id", room_id))
            .await {
                Ok(queried) => queried,
                Err(e) => {log::error!("Failed to query messages: fn catch_up, error: {:?}", e);
                return None}
            };
        let basic_messages: Vec<BasicMessage> = match response.take(0)
            {
                Ok(retrieved) => retrieved,
                Err(e) => {log::error!("Failed to get messages from query: fn catch_up, error: {:?}", e);
                return None}
            };
        let user_messages: Vec<UserMessage> = basic_messages.into_iter().map(UserMessage::Basic).collect();
        Some(user_messages)
    }

    pub async fn authenticate_user(&self, login_data: &LoginForm) -> Option<String> {
        let query = "SELECT * FROM users WHERE login_username = $login_username;";
        let mut response = match self.db
            .query(query)
            .bind(("login_username", login_data.username.clone()))
            .await {
                Ok(queried) => queried,
                Err(e) => {log::error!("Failed to query for user: fn authenticate_user, error: {:?}", e);
                return None}
            };
        let result: Option<UserData> = match response.take(0)
            {
                Ok(user) => user,
                Err(e) => {log::error!("Failed to get user data: fn authenticate_user, error: {:?}", e);
                return None}
            };

        match result {
            Some(user_data) => {
                if bcrypt::verify(login_data.password.clone(), &user_data.hashed_password).unwrap_or(false) {
                    Some(user_data.user_id)
                } else {
                    None
                }
            },
            None => {
                None
            }
        }
    }

    pub async fn valid_user_credentials(&self, signup_data: &LoginForm) -> bool {
        let result: Option<UserData> = match self.db.select(("logins", &signup_data.username))            .await {
            Ok(retrieved) => retrieved,
            Err(e) => {log::error!("Failed to get user : fn valid_user_credentials, error: {:?}", e);
            return false}
        };

        match result {
            Some(_) => {
                false
            },
            None => {
                signup_data.validate().is_ok()
            }
        }
    }
}