use actix::Addr;
use surrealdb::{Result, Surreal};
use surrealdb::engine::remote::ws::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use validator::Validate;
use crate::structs::{Room, UserData, LoginForm};
use crate::message_structs::*;
use crate::websocket::{WsActor, WsMessage};

pub struct AppState {
    pub db: Arc<Surreal<Client>>,
    pub channels: Arc<Mutex<HashMap<String, Room>>>,
    pub actor_registry: Arc<Mutex<HashMap<String, HashMap<String, Addr<WsActor>>>>>,
    pub main_room_id: String,
}

impl AppState {
    pub async fn broadcast_message(&self, message: String, room_id: String, user_id: String) {
        let query = "SELECT * FROM rooms WHERE room_id = $room_id;";
        let mut response = self.db.query(query)
            .bind(("room_id", room_id))
            .await.expect("Failed to get users in a partiucalr room: fn broadcast_message");
        let rooms: Vec<Room> = response.take(0).expect("Failed to Deserialize room query data");
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

    pub async fn catch_up(&self, room_id: &str) -> Result<Vec<UserMessage>> {
        let query = "SELECT * FROM messages WHERE room_id = $room_id ORDER BY timestamp ASC;";
        let mut response = self.db.query(query).bind(("room_id", room_id))
            .await?;
        let basic_messages: Vec<BasicMessage> = response.take(0)?;
        let user_messages: Vec<UserMessage> = basic_messages.into_iter().map(UserMessage::Basic).collect();
        Ok(user_messages)
    }

    pub async fn authenticate_user(&self, login_data: &LoginForm) -> Option<String> {
        let query = "SELECT * FROM users WHERE login_username = $login_username;";
        let mut response = self.db
            .query(query)
            .bind(("login_username", login_data.username.clone()))
            .await.expect("Failed to query for a user");
        let result: Option<UserData> = response.take(0).expect("failed to deserilize user data: fn authenticate_user");

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
        let result: Option<UserData> = self.db.select(("logins", &signup_data.username)).await.expect("Failed to query user: fn valid_user_credentials");

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