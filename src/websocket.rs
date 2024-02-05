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
use crate::appstate::AppState;
use crate::structs::{Room, User, UserData};
use crate::message_structs::*;

pub async fn get_messages(app_state: Arc<AppState>, actor_addr: Addr<WsActor>, room_id: String) {
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
    let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
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
        let mut actor_registry = self.state.actor_registry.lock().unwrap();
        match actor_registry.get_mut(&self.user_id) {
            Some(hashmap) => {
                hashmap.insert(self.ws_id.clone(), ctx.address());
            },
            None => {
                let mut hashmap: HashMap<String, Addr<WsActor>> = HashMap::new();
                hashmap.insert(self.ws_id.clone(), ctx.address());
                actor_registry.insert(self.user_id.clone(), hashmap);
            },
        }
        let db = self.state.db.clone();
        let app_state = self.state.clone();
        let room_id = self.current_room.clone();
        let user_id = self.user_id.clone();
        let user_info = UserInfo::new(self.user_id.clone(), self.ws_id.clone(), self.username.clone());
        ctx.spawn(actix::fut::wrap_future(get_users(db.clone(), ctx.address(), room_id.clone(), user_info)));
        ctx.spawn(actix::fut::wrap_future(get_messages(app_state, ctx.address(), room_id)));
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

pub async fn get_users(db: Arc<Surreal<Client>>, actor_addr: Addr<WsActor>, room_id: String, user_info: UserInfo) {
    let query = "SELECT user_id, username FROM users WHERE $room_id IN rooms;";
    let mut response = db.query(query)
        .bind(("room_id", room_id))
        .await.expect("Failed to execute query to get users in a particular room");
    let users: Vec<User> = response.take(0).expect("Failed to Deserialize user data from db: fn get_users");
    let user_map: HashMap<String, String> = users.into_iter()
        .map(|user| (user.user_id, user.username))
        .collect();
    let init_message = UserMessage::Initialization(InitMessage::new(user_info.user_id, user_info.ws_id, user_info.username, user_map));
    let serialized = serde_json::to_string(&init_message).unwrap();
    actor_addr.do_send(WsMessage(serialized));
}

pub async fn check_and_update_username(user_id: String, current_username: String, new_username: String, state: Arc<AppState>) {
    let query = "SELECT username FROM users WHERE username = $username;";
    if let Ok(mut response) = state.db.query(query).bind(("username", new_username.clone())).await {
        let result: Option<String> = response.take((0, "username")).expect("Failed to get user data: fn check_and_update_username");
        match result {
            Some(_) => println!("Username already used"),
            None =>  {
                let query = "UPDATE users SET username = $new_username WHERE username = $username;";
                state.db.query(query)
                    .bind(("new_username", new_username.clone()))
                    .bind(("username", current_username))
                    .await.expect("Failed to update username");
                let message = UserMessage::UsernameChange(UsernameChangeMessage::new(user_id.clone(), new_username.clone()));
                let serialized_msg = serde_json::to_string(&message).unwrap();
                state.broadcast_message(serialized_msg, state.main_room_id.clone(), user_id).await;
            }
        }
    } else {
        eprintln!("Failed to query database");
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<UserMessage>(&text) {
                Ok(message) => {
                    match message {
                        UserMessage::Basic(basic_message) => {
                            if basic_message.sender_id == self.user_id {
                                let app_state = self.state.clone();
                                let now = Utc::now();
                                let basic_message = BasicMessage {
                                    content: basic_message.content, 
                                    sender_id: basic_message.sender_id, 
                                    timestamp: now.timestamp() as u64, 
                                    message_id: Uuid::new_v4().to_string().replace('-', ""), 
                                    room_id: self.current_room.clone(), 
                                    ws_id: self.ws_id.clone()
                                };
                                actix::spawn(async move {
                                    let _: Option<BasicMessage> = app_state.db.create(("messages", basic_message.message_id.clone()))
                                        .content(basic_message.clone())
                                        .await.expect("Failed to upload message to db");
                                    let serialized_msg = serde_json::to_string(&UserMessage::Basic(basic_message.clone())).unwrap();
                                    app_state.broadcast_message(serialized_msg, basic_message.room_id, basic_message.sender_id).await;
                                });
                            } else {
                                println!("Invalid Access")
                            }
                        },
                        UserMessage::UsernameChange(username_change_message) => {
                            let new_username = username_change_message.new_username;
                            let username = self.username.clone();
                            let state = self.state.clone();
                            let user_id = self.user_id.clone();
                            ctx.spawn(actix::fut::wrap_future(check_and_update_username( user_id, username, new_username, state)));
                        },
                        UserMessage::CreateRoomChange(create_room_change_message) => {
                            let room_id = Uuid::new_v4().to_string().replace('-', "");
                            let room_name = create_room_change_message.room_name;
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
                        },
                        UserMessage::ChangeRoom(change_room_message) => {
                            let room_id = change_room_message.room_id;
                            let app_state = self.state.clone();
                            let actor_addr = ctx.address().clone();
                            ctx.spawn(actix::fut::wrap_future(get_messages(app_state, actor_addr, room_id)));
                        },
                        UserMessage::UserRemoval(user_removal_message) => {
                            let app_state = self.state.clone();
                            actix::spawn(async move {
                                let query =  "UPDATE rooms SET users -= $removed_user WHERE room_id = $room_id;";
                                if let Err(e) = app_state.db.query(query)
                                    .bind(("removed_user", user_removal_message.removed_user))
                                    .bind(("room_id", user_removal_message.room_id))
                                    .await {
                                    eprintln!("Error removing from room: {:?}", e);
                                }
                            });
                        }
                        _ => {}
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
            .await.expect("Error in Finding User");
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
