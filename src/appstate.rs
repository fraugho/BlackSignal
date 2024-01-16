use actix::{Addr};

use surrealdb::{Result, Surreal};
use surrealdb::engine::remote::ws::Client;

use std::collections::HashMap;

use std::sync::{Arc, Mutex};

use validator::Validate;

use serde_json::json;

use crate::structs::{Room, UserMessage, UserData, LoginForm};  
use crate::websocket::{WsActor, WsMessage}; 

pub struct AppState {
    pub db: Arc<Surreal<Client>>,
    pub channels: Arc<Mutex<HashMap<String, Room>>>,
    pub actor_registry: Arc<Mutex<HashMap<String, Addr<WsActor>>>>,
    pub main_room_id: String,
}

impl AppState {
    pub async fn broadcast_message(&self, message: String, room_id: String) {
        // Querying connections to get a list of UUIDs


        for client in self.actor_registry.lock().unwrap().values(){
            client.do_send(WsMessage(message.clone()));
        }
    }

    pub async fn join_main_room(&self, username: String, user_id :String){
        let message = json!({
            "type": "new_user_joined",
            "username": username,
            "user_id": user_id,
        });
        let serialized_msg = serde_json::to_string(&message).unwrap();
        self.broadcast_message(serialized_msg, self.main_room_id.clone()).await;
    }

    pub async fn catch_up(&self, room_id: &str) -> Result<Vec<UserMessage>> {

        let query = "SELECT * FROM messages WHERE room_id = $room_id ORDER BY timestamp ASC;";
    
        let mut response = self.db.query(query)
            .bind(("room_id", room_id))
            .await?;
    
        let messages: Vec<UserMessage> = response.take(0)?;
    
        Ok(messages)
    }  

    pub async fn authenticate_user(&self, login_data: &LoginForm) -> Option<String>{
        let query = "SELECT * FROM users WHERE login_username = $login_username;";
        let mut response = self.db
            .query(query)
            .bind(("login_username", login_data.username.clone()))
            .await.expect("aaaah");
        let result: Option<UserData> = response.take(0).expect("cool");
        //let result: Option<UserData> = self.db.select(("logins", username)).await.expect("something");
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

    pub async fn valid_user_credentials(&self, signup_data: &LoginForm) -> bool{
        let result: Option<UserData> = self.db.select(("logins", &signup_data.username)).await.expect("something");
        match result {
            Some(_) => {
                eprintln!("bad");
                false
            },
            None => {
                //eprintln!("validate: {}", signup_data.validate().is_ok());
                signup_data.validate().is_ok()
            }
        }
        
    }

}