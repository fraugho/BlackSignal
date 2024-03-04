use crate::appstate::AppState;
use crate::message_structs::*;
use crate::structs::{Room, User, UserData};
use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_session::Session;
use actix_web::{web, HttpResponse, Error};
use actix_web_actors::ws;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use uuid::Uuid;
use serde_json::json;
use std::time::{Instant, Duration};

const MESSAGE_TOKENS: u32 = 100;
const TIME_FRAME: Duration = Duration::from_secs(10);

pub async fn get_messages(
    app_state: Arc<AppState>, 
    actor_addr: Addr<WsActor>, 
    room_id: String) {
    if let Some(messages) = app_state.catch_up(&room_id).await {
        for message in messages {
            let serialized_msg = serde_json::to_string(&message).unwrap();
            actor_addr.do_send(WsMessage(serialized_msg));
        }
    }
}

pub async fn change_to_online(db: Arc<Surreal<Client>>, user_id: String) {
    let query = "UPDATE users SET status = 'Online' WHERE user_id = $user_id;";
    if let Err(e) = db.query(query).bind(("user_id", user_id)).await {
        log::error!(
            "Failed to change user to online in db: fn change_to_online, error: {:?}",
            e
        );
    }
}

pub async fn change_to_offline(db: Arc<Surreal<Client>>, user_id: String) {
    let query = "UPDATE users SET status = 'Offline' WHERE user_id = $user_id;";
    if let Err(e) = db.query(query).bind(("user_id", user_id)).await {
        log::error!(
            "Failed to change user to offline in db: fn change_to_offline, error: {:?}",
            e
        );
    }
}

pub struct WsActor {
    pub ws_id: String,
    pub user_id: String,
    pub username: String,
    pub current_room: String,
    pub rooms: Vec<String>,
    pub state: Arc<AppState>,
    pub request_token_count: u32,
    pub start_time: Instant,
}

impl WsActor {
    fn reset_rate_limit(&mut self) {
        self.request_token_count = 10;
        self.start_time = Instant::now();
    }
    
    fn check_and_update_rate_limit(&mut self) -> bool {
        let elapsed = self.start_time.elapsed();
        
        // Check if the current period has exceeded the time frame
        if elapsed > TIME_FRAME {
            // Time frame exceeded, reset rate limiting counters
            self.reset_rate_limit();
            true
        } else {
            false
        }
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
            }
            None => {
                let mut hashmap: HashMap<String, Addr<WsActor>> = HashMap::new();
                hashmap.insert(self.ws_id.clone(), ctx.address());
                actor_registry.insert(self.user_id.clone(), hashmap);
            }
        }
        let db = self.state.db.clone();
        let app_state = self.state.clone();
        let room_id = self.current_room.clone();
        let user_id = self.user_id.clone();
        let user_info = UserInfo::new(
            self.user_id.clone(),
            self.ws_id.clone(),
            self.username.clone(),
        );
        ctx.spawn(actix::fut::wrap_future(get_users(
            db.clone(),
            ctx.address(),
            room_id.clone(),
            user_info,
        )));
        ctx.spawn(actix::fut::wrap_future(get_messages(
            app_state,
            ctx.address(),
            room_id,
        )));
        ctx.spawn(actix::fut::wrap_future(change_to_online(db, user_id)));
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let user_id = self.user_id.clone();
        let db = self.state.db.clone();
        let mut actor_registry = self.state.actor_registry.lock().unwrap();
        if let Some(hashmap) = actor_registry.get_mut(&self.user_id.clone()) {
            hashmap.remove(&self.ws_id.clone());
        }
        actix::spawn(async move { change_to_offline(db, user_id).await });
    }

}

pub async fn add_user_to_room(
    user_id: String, 
    room_id: String, 
    db: Arc<Surreal<Client>>) {
    let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
    if let Err(e) = db
        .query(query)
        .bind(("user_id", user_id))
        .bind(("room_id", room_id))
        .await
    {
        log::error!(
            "Failed to add user to room: fn add_user_to_room, error: {:?}",
            e
        );
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

pub async fn delete_message(message: DeletionMessage, sender_id: String, room_id: String, state: Arc<AppState>) {
    let query = "SELECT * FROM messages WHERE sender_id = $sender_id AND message_id = $message_id;";
    let mut response = match state.db.query(query).bind(("sender_id", sender_id.clone())).bind(("message_id", message.message_id.clone())).await {
        Ok(x) => x,
        Err(e) => {
            log::error!(
                "Failed to delete message: fn delete_message, error: {:?}",
                e
            );
            return
        }
    };
    let _: Option<BasicMessage> = match response.take(0) {
        Ok(x) => x,
        Err(e) => {log::error!("Failed to delete message: fn delete_message, error: {:?}", e);
            return}
    };
    let _: Option<BasicMessage> = match state.db.delete(("messages", message.message_id.clone())).await {
        Ok(x) => x,
        Err(e) => {
            log::error!(
                "Failed to delete message: fn delete_message, error: {:?}",
                e
            );
            None
        }
    };
    let serialized_message = match serde_json::to_string(&UserMessage::Deletion(message)){
        Ok(x) => x,
        Err(e) => {log::error!("Failed to delete message: fn delete_message, error: {:?}", e);
        return},
    };
    state.broadcast_message(serialized_message, room_id, sender_id).await;
}

pub async fn get_users(
    db: Arc<Surreal<Client>>,
    actor_addr: Addr<WsActor>,
    room_id: String,
    user_info: UserInfo,
) {
    let query = "SELECT user_id, username FROM users WHERE $room_id IN rooms;";
    let mut response = match db.query(query).bind(("room_id", room_id)).await {
        Ok(retrieved) => retrieved,
        Err(e) => {
            log::error!(
                "Failed to query users that are in requested room: fn get_users, error: {:?}",
                e
            );
            return;
        }
    };
    let users: Vec<User> = match response.take(0) {
        Ok(user) => user,
        Err(e) => {
            log::error!(
                "Failed to get users that are in requested room: fn get_users, error: {:?}",
                e
            );
            return;
        }
    };
    let user_map: HashMap<String, String> = users
        .into_iter()
        .map(|user| (user.user_id, user.username))
        .collect();
    let init_message = UserMessage::Initialization(InitMessage::new(
        user_info.user_id,
        user_info.ws_id,
        user_info.username,
        user_map,
    ));
    let serialized = serde_json::to_string(&init_message).unwrap();
    actor_addr.do_send(WsMessage(serialized));
}

pub async fn check_and_update_username(
    user_id: String,
    current_username: String,
    new_username: String,
    state: Arc<AppState>,
    message: UserMessage,
) -> Result<HttpResponse, Error> {
    let query = "SELECT username FROM users WHERE username = $username;";
    if let Ok(mut response) = state
        .db
        .query(query)
        .bind(("username", new_username.clone()))
        .await
    {
        let result: Option<String> = match response.take((0, "username")) {
            Ok(retrieved) => retrieved,
            Err(e) => {
                log::error!(
                    "Failed to get user: fn check_and_update_username, error: {:?}",
                    e
                );
                return Ok(HttpResponse::InternalServerError().json(json!({"error": "Internal DB Error"})));
            }
        };
        match result {
            Some(_) => Ok(HttpResponse::BadRequest().json(json!({"error": "Username Already In Use"}))),
            None => {
                let query = "UPDATE users SET username = $new_username WHERE username = $username;";
                if let Err(e) = state
                    .db
                    .query(query)
                    .bind(("new_username", new_username.clone()))
                    .bind(("username", current_username)).await{
                        log::error!(
                            "Failed to update username: fn check_and_update_username, error: {:?}",
                            e
                        );
                        return Ok(HttpResponse::InternalServerError().json(json!({"error": "Internal DB Error"})));
                    };

                let serialized_msg = serde_json::to_string(&message).unwrap();
                state
                    .broadcast_message(serialized_msg, state.main_room_id.clone(), user_id)
                    .await;
                Ok(HttpResponse::Ok().json(json!({"message": "Username updated successfully"}))
            )
            }
        }
    } else {
        Ok(HttpResponse::BadRequest().json(json!({"error": "Failed to query database"})))
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(
        &mut self,
        msg: std::result::Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        if !self.check_and_update_rate_limit() && self.request_token_count == 0  {
            return;
        }
        match self.request_token_count.checked_sub(1){
            Some(result) => self.request_token_count = result,
            None => println!("Underflow occurred"),
        }
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<UserMessage>(&text) {
                Ok(message) => match message {
                    UserMessage::TSBasic(ts_basic_message) => {
                        let app_state = self.state.clone();
                        let now = Utc::now();
                        let basic_message = BasicMessage {
                            content: ts_basic_message.content,
                            sender_id: self.user_id.clone(),
                            timestamp: now.timestamp() as u64,
                            message_id: Uuid::new_v4().to_string().replace('-', ""),
                            room_id: self.current_room.clone(),
                            ws_id: self.ws_id.clone(),
                        };
                        actix::spawn(async move {
                            let _: Option<BasicMessage> = match app_state
                                .db
                                .create(("messages", basic_message.message_id.clone()))
                                .content(basic_message.clone())
                                .await {
                                    Ok(retrieved) => retrieved,
                                    Err(e) => {log::error!("Failed to create message in db: fn handle, error: {:?}", e);
                                    return}
                                };
                            let serialized_msg = match serde_json::to_string(&UserMessage::Basic(basic_message.clone(),)){
                                Ok(serialized) => serialized,
                                Err(e) => {log::error!("Failed to create message in db: fn handle, error: {:?}", e);
                                return}
                            };
                            app_state
                                .broadcast_message(
                                    serialized_msg,
                                    basic_message.room_id,
                                    basic_message.sender_id,
                                )
                                .await;
                        });
                    }
                    UserMessage::Deletion(message) => {
                        let sender_id = self.user_id.clone();
                        let state = self.state.clone();
                        let room_id = self.current_room.clone();
                        ctx.spawn(actix::fut::wrap_future(delete_message(message, sender_id, room_id, state)));
                        
                    }
                    UserMessage::CreateRoomChange(create_room_change_message) => {
                        let room_id = Uuid::new_v4().to_string().replace('-', "");
                        let room_name = create_room_change_message.room_name;
                        let app_state = self.state.clone();
                        self.rooms.push(room_id.clone());
                        let mut users = HashSet::new();
                        users.insert(self.user_id.clone());
                        actix::spawn(async move {
                            let _: Vec<Room> = match app_state
                                .db
                                .create("rooms")
                                .content(Room {
                                    name: room_name,
                                    room_id,
                                    users,
                                })
                                .await {
                                    Ok(retrieved) => retrieved,
                                    Err(e) => {log::error!("Failed to create room in db: fn handle, error: {:?}", e);
                                    return}
                                };
                        });
                    }
                    UserMessage::ChangeRoom(change_room_message) => {
                        let room_id = change_room_message.room_id;
                        let app_state = self.state.clone();
                        let actor_addr = ctx.address().clone();
                        ctx.spawn(actix::fut::wrap_future(get_messages(
                            app_state, actor_addr, room_id,
                        )));
                    }
                    UserMessage::UserRemoval(user_removal_message) => {
                        let app_state = self.state.clone();
                        actix::spawn(async move {
                            let query =
                                "UPDATE rooms SET users -= $removed_user WHERE room_id = $room_id;";
                            if let Err(e) = app_state
                                .db
                                .query(query)
                                .bind(("removed_user", user_removal_message.removed_user))
                                .bind(("room_id", user_removal_message.room_id))
                                .await
                            {
                                log::error!("Error removing from room: {:?}", e);
                            }
                        });
                    }
                    _ => {}
                },
                Err(e) => log::error!("Error processing message: {:?}", e),
            }
        }
    }
}

pub async fn ws_index(
    req: actix_web::HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::Error> {
    let main_room_id = state.main_room_id.clone();
    if let Some(user_id) = session.get::<String>("key").unwrap() {
        let query = "SELECT * FROM users WHERE user_id = $user_id;";
        let mut response = match state
            .db
            .query(query)
            .bind(("user_id", user_id.clone()))
            .await {
                Ok(retrieved) => retrieved,
                Err(e) => {log::error!("Failed to query user data: fn ws_index, error: {:?}", e);
                    session.purge();
                    return Ok(HttpResponse::Found()
                        .append_header(("LOCATION", "/login"))
                        .finish());
                }
            };
        let user_query: Option<UserData> = match response
            .take(0){
                Ok(retrieved) => retrieved,
                Err(e) => {log::error!("Failed to get user data: fn ws_index, error: {:?}", e);
                    session.purge();
                    return Ok(HttpResponse::Found()
                        .append_header(("LOCATION", "/login"))
                        .finish());
                }
            };
        match user_query {
            Some(user) => {
                let ws_actor = WsActor {
                    user_id,
                    ws_id: Uuid::new_v4().to_string().replace('-', ""),
                    username: user.username,
                    current_room: main_room_id.clone(),
                    rooms: user.rooms,
                    state: state.into_inner().clone(),
                    request_token_count: MESSAGE_TOKENS,
                    start_time: Instant::now(),
                };
                return ws::start(ws_actor, &req, stream);
            }
            None => {
                session.purge();
                return Ok(HttpResponse::Found()
                    .append_header(("LOCATION", "/login"))
                    .finish());
            }
        }
    }
    return Ok(HttpResponse::Found()
        .append_header(("LOCATION", "/login"))
        .finish());
}