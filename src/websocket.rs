use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{web, HttpResponse};
use actix_web_actors::ws;
use actix_session::Session;
use actix::Message;

use surrealdb::{Result, Surreal};
use surrealdb::engine::remote::ws::Client;

use serde::{Serialize, Deserialize};

use uuid::Uuid;

use chrono::Utc;

use std::collections::{HashMap, HashSet};

use std::sync::Arc;

use serde_json::json;

use crate::appstate::AppState; 
use crate::structs::{UserMessage, MessageTypes, Room, IncomingMessage, User, UserData};

pub async fn get_messages(app_state: Arc<AppState>,  actor_addr: Addr<WsActor>, room_id: String) {
    match app_state.catch_up(&room_id).await {
        Ok(messages) => {
            for message in messages {

                let serialized_msg = serde_json::to_string(&message).unwrap();
                
                actor_addr.do_send(WsMessage(serialized_msg));
            }
        }
        Err(e) => {
            eprintln!("Error catching up messages: {:?}", e);
        }
    }
}

pub async fn change_to_online(db: Arc<Surreal<Client>>, user_id: String) {
    let query = "UPDATE users SET status = 'Online' WHERE user_id = $user_id;";
    db.query(query)
        .bind(("user_id", user_id))
        .await.expect("Failed to update user status to online");
}

pub async fn change_to_offline(db: Arc<Surreal<Client>>, user_id: String) {
    let query = "UPDATE users SET status = 'Offline' WHERE user_id = $user_id;";
    db.query(query)
        .bind(("user_id", user_id))
        .await.expect("Failed to update user status to offline");
}

pub struct WsActor {
    pub ws_id: String,
    pub user_id: String,
    pub username: String,
    pub current_room: String,
    pub rooms: Vec<String>, 
    pub state: Arc<AppState>, 
}

impl WsActor {
    async fn delete_message(&self, message_id: String) -> Result<()> {
        let _: Option<UserMessage> = self.state.db.delete(("messages", message_id))
            .await.expect("error deleting_message");

        Ok(())
    }
}

pub async fn add_user_to_room(user_id: String, room_id: String, db: Arc<Surreal<Client>>) {
    let query =  "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
        if let Err(e) = db.query(query)
            .bind(("user_id", user_id))
            .bind(("room_id", room_id))
            .await {
            eprintln!("Error adding to room: {:?}", e);
    }
}


impl Actor for WsActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {


        //registers ws actor
        let mut actor_registry= self.state.actor_registry.lock().unwrap();
        let db = self.state.db.clone();


        match actor_registry.get_mut(&self.user_id.clone()) {
            Some(hashmap) => {
                hashmap.insert(self.ws_id.clone(), ctx.address());
            }, 
            None => {
                let mut hashmap: HashMap<String, Addr<WsActor>> = HashMap::new();
                hashmap.insert(self.ws_id.clone(), ctx.address());
                actor_registry.insert(self.user_id.clone(), hashmap);},
        }
        let app_state = self.state.clone();
        let room_id = self.current_room.clone();
        let actor_addr = ctx.address();

        let user_id = self.user_id.clone();

        let actor_addr_clone = actor_addr.clone();
        let actor_addr_clone2 = actor_addr.clone();
        


        ctx.spawn(actix::fut::wrap_future(get_users(db.clone(), actor_addr_clone, room_id.clone())));

        let init_message = json!({
            "type": "init",
            "user_id": self.user_id,
            "ws_id": self.ws_id,
            "username": self.username,
        });

        ctx.text(init_message.to_string());

        ctx.spawn(actix::fut::wrap_future(get_messages(app_state, actor_addr_clone2, room_id)));
        ctx.spawn(actix::fut::wrap_future(change_to_online(db, user_id)));
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {

        let user_id = self.user_id.clone();

        let db = self.state.db.clone();

        let mut actor_registry = self.state.actor_registry.lock().unwrap();
        if let Some(hashmap) = actor_registry.get_mut(&self.user_id.clone()) {
            hashmap.remove(&self.ws_id.clone());
        }

        actix::spawn(async move {
            change_to_offline(db, user_id).await
        });
    }
}

pub struct WsMessage(pub String);

impl actix::Message for WsMessage {
    type Result = ();
}

impl Handler<WsMessage> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        // Always send the message to the client, including the sender
        ctx.text(msg.0);
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUsernameMsg(String);

impl Message for UpdateUsernameMsg {
    type Result = ();
}

impl Handler<UpdateUsernameMsg> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: UpdateUsernameMsg, ctx: &mut Self::Context) -> Self::Result {
        let init_message = json!({
            "type": "init",
            "username": msg.0
        });

        
        self.username = msg.0;
    }
}

#[derive(Serialize, Deserialize)]
pub struct SendUserStruct {
    user_hashmap: HashMap<String, String>,
    message_type : String,
}

#[derive(Serialize, Deserialize)]
pub struct SendUsers(String);

impl Message for SendUsers {
    type Result = ();
}

impl Handler<SendUsers> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: SendUsers, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0)
        
    }
}

pub async fn get_users(db: Arc<Surreal<Client>>,  actor_addr: Addr<WsActor>, room_id: String) {

    let query = "SELECT user_id, username FROM users WHERE $room_id IN rooms;";

    let mut response = db.query(query)
        .bind(("room_id", room_id))
        .await.expect("Failed to execute query to get users in a particular room");

    let users: Vec<User> = response.take(0).expect("Failed to Deserialize user data from db: fn get_users");

    let users_map: HashMap<String, String> = users.into_iter()
        .map(|user| (user.user_id, user.username))
        .collect();


    let send_user = SendUserStruct {
        user_hashmap: users_map,
        message_type: "user_list".to_string(),
    };

    let serialized = serde_json::to_string(&send_user).unwrap();

    actor_addr.do_send(SendUsers(serialized));

}

pub async fn check_and_update_username(db: Arc<Surreal<Client>>, user_id: String, current_username: String, new_username: String, actor_addr: Addr<WsActor>, state: Arc<AppState>) {
    let query = "SELECT username FROM users WHERE username = $username;";
    if let Ok(mut response) = db.query(query).bind(("username", new_username.clone())).await {
        let x: Option<String> = response.take((0, "username")).expect("Failed to get user data: fn check_and_update_username");

        match x {
            Some(_) => println!("Username already used"),
            None =>  {
                let query = "UPDATE users SET username = $new_username WHERE username = $username;";
                db.query(query)
                    .bind(("new_username", new_username.clone()))
                    .bind(("username", current_username.clone()))
                    .await.expect("Failed to update username");
                //let message = UpdateUsernameMsg(new_username);
                let message = json!({
                    "type": "update_username",
                    "new_username": new_username,
                    "sender_id": user_id,
                });
                let serialized_msg = serde_json::to_string(&message).unwrap();
                state.broadcast_message(serialized_msg, state.main_room_id.clone(), user_id.clone()).await;
                actor_addr.do_send(UpdateUsernameMsg(new_username));
            }
                
        }
    } else {
        eprintln!("Failed to query database");
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {                
                Ok(incoming_message) => {
                    if incoming_message.sender_id == self.user_id {
                        match incoming_message.message_type {
                            MessageTypes::SetUsername => {
                                let new_username = incoming_message.content;
                                let username = self.username.clone();
                                let db = self.state.db.clone();
                                let actor_addr = ctx.address();
                                let state = self.state.clone();
                                let user_id = self.user_id.clone();
                            
                                // Spawn the async function
                                ctx.spawn(actix::fut::wrap_future(check_and_update_username(db, user_id, username, new_username, actor_addr, state,  )));
                            },
                            
                            MessageTypes::CreateRoom => {
                                let room_id = Uuid::new_v4().to_string().replace('-', "");
                                let room_name = incoming_message.content;
                                let app_state = self.state.clone();

                                self.rooms.push(room_id.clone());

                                let mut users = HashSet::new();
                                users.insert(self.user_id.clone());
                                
    
                                actix::spawn(async move {
                                    let _ : Vec<Room> = app_state.db.create("rooms")
                                        .content(Room {
                                            name: room_name,
                                            room_id,
                                            users,
                                        })
                                        .await.expect("Failed to create room");
                                });
                            }
                            MessageTypes::AddToRoom => {
                                let room_id = incoming_message.content.clone();
                                let user_id = incoming_message.content;
                                let app_state = self.state.clone();
    
                                actix::spawn(async move {
                                    let query =  "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
                                    if let Err(e) = app_state.db.query(query)
                                        .bind(("user_id", user_id))
                                        .bind(("room_id", room_id))
                                        .await {
                                        eprintln!("Error adding to room: {:?}", e);
                                    }
                                });
                            }
                            MessageTypes::ChangeRoom => {
                                let room_id = incoming_message.content;
                                let user_id = self.username.clone();
                                let app_state = self.state.clone();
                                let actor_addr = ctx.address().clone();

                                ctx.spawn(actix::fut::wrap_future(get_messages(app_state, actor_addr, room_id)));
                            }
                            MessageTypes::RemoveFromRoom => {}
                            MessageTypes::Basic => {
                                let content = incoming_message.content;
            
                                let app_state = self.state.clone();
                                let sender_id = self.user_id.clone();
                                let room_id = self.rooms[0].clone();
                                let now = Utc::now();
                                let timestamp = now.timestamp() as u64;
                                let message_id = Uuid::new_v4().to_string().replace('-', "");
                                let ws_id = self.ws_id.clone();
                                // Create the message only once, reusing the variables directly
                                let message = UserMessage {
                                    message_id: message_id.clone(),
                                    content: content.clone(),
                                    timestamp,
                                    ws_id: ws_id.clone(),
                                    sender_id: sender_id.clone(),
                                    room_id: room_id.clone(), 
                                    message_type: MessageTypes::Basic,
                                };
                    
                                actix::spawn(async move {
            
                                    let _: Option<UserMessage> = app_state.db.create(("messages", message_id.clone()))
                                        .content(UserMessage {
                                            message_id,
                                            content,
                                            timestamp,
                                            ws_id,
                                            sender_id: sender_id.clone(),
                                            room_id: room_id.clone(),
                                            message_type: MessageTypes::Basic,
                                        })
                                        .await.expect("Failed to upload message to db");
                    
                                    let serialized_msg = serde_json::to_string(&message).unwrap();
                                    app_state.broadcast_message(serialized_msg, room_id, sender_id.clone()).await;
                                });
                                },
                            }
                    } 
                }

                Err(e) => eprintln!("Error processing message: {:?}", e),
            }
        }
    }
}

pub async fn ws_index(req: actix_web::HttpRequest, stream: web::Payload, state: web::Data<AppState>, session: Session) -> std::result::Result<HttpResponse, actix_web::Error> {
    let main_room_id = state.main_room_id.clone();
    if let Some(user_id) = session.get::<String>("key").unwrap(){
        let query = "SELECT * FROM users WHERE user_id = $user_id;";
        let mut response = state.db.query(query)
            .bind(("user_id", user_id.clone()))
            .await.expect("aaaah");
        let user_query: Option<UserData> = response.take(0).expect("Failed to get user data: fn ws_index");

        match user_query {
            Some(user) => {

                let ws_actor = WsActor {
                    user_id,
                    ws_id: Uuid::new_v4().to_string().replace('-', ""),
                    username: user.username,
                    current_room: main_room_id.clone(),
                    rooms: user.rooms,
                    state: state.into_inner().clone(),
                };
            
                return ws::start(ws_actor, &req, stream)
            },
            None => {
                
                session.purge();  
                return Ok(HttpResponse::Found().append_header(("LOCATION", "/login")).finish());
            }
        }
    }
    return Ok(HttpResponse::Found().append_header(("LOCATION", "/login")).finish());
    
}