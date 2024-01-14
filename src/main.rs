use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use actix_web_actors::ws;
use actix_session::{Session, CookieSession};
use actix::fut::ActorFutureExt;
use actix::Message;

use names::{Generator, Name};

use bcrypt::{hash, DEFAULT_COST};

use surrealdb::{Result, Surreal};
use surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::{Client, Ws};

use serde::{Serialize, Deserialize};

use uuid::Uuid;

use chrono::Utc;

use std::fmt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::fs::read_to_string;

use serde_json::json;

use validator::Validate;
//use anyhow::Result;

/*
Tables in DB
Users
Connections
Messages
*/



pub struct AppState {
    db: Arc<Surreal<Client>>,
    channels: Arc<Mutex<HashMap<String, Room>>>,
    actor_registry: Arc<Mutex<HashMap<String, Addr<WsActor>>>>,
    main_room_id: String,
}

impl AppState {
    pub async fn add_ws(&self, room_ids: Vec<String>, user_id: String, address: Addr<WsActor>) -> Result<()> {
        //self.actor_registry.lock().unwrap().insert(user_id, address);
        for room_id in room_ids {
            todo!()
        }

        Ok(())
    }
    pub async fn remove_ws(&self, room_ids: Vec<String>, user_id: String) -> Result<()> {
        for room_id in room_ids {
            let query = format!(
                "DELETE FROM connections WHERE user_id = {} RETURN BEFORE;",
                user_id
            );

            //let connection:Option<Connection> = self.connections.delete((room_id, user_id)).await?;
            self.db.query(&query).await?;
        }

        Ok(())
    }

    pub async fn broadcast_message(&self, message: String, sender_id: String, room_id: String) {
        // Querying connections to get a list of UUIDs
        for client in self.actor_registry.lock().unwrap().values(){
            client.do_send(WsMessage(message.clone(), sender_id.clone()));
        }
    }

    pub async fn catch_up(&self, room_id: &str) -> Result<Vec<UserMessage>> {

        let sql = r#"SELECT * FROM messages ORDER BY timestamp ASC"#;
    
        let mut response = self.db.query(sql)
            .await?;
    
        let messages: Vec<UserMessage> = response.take(0)?;
    
        Ok(messages)
    }  



    pub async fn get_rooms(&self, user_id: &str) -> Result<Vec<String>> {
        let sql = format! {"SELECT room_id FROM connections WHERE id = '{}'", user_id};
    
        let mut response = self.db.query(sql)
            .await?;
    
        let rooms: Vec<String> = response.take(0)?;
    
        Ok(rooms)
    }

    async fn authenticate_user(&self, login_data: &LoginForm) -> Option<String>{
        let query = format!{"SELECT * FROM users WHERE login_username = '{}';", login_data.username};
        let mut response = self.db.query(query).await.expect("aaaah");
        let result: Option<UserData> = response.take(0).expect("cool");
        //let result: Option<UserData> = self.db.select(("logins", username)).await.expect("something");
        match result {
            Some(user_data) => {
                if bcrypt::verify(login_data.password.clone(), &user_data.hashed_password).unwrap_or(false) {
                    Some(user_data.uuid)
                } else {
                    None
                }
            },
            None => {
                None
            }
        }
    }

    async fn valid_user_credentials(&self, signup_data: &LoginForm) -> bool{
        let result: Option<UserData> = self.db.select(("logins", &signup_data.username)).await.expect("something");
        match result {
            Some(_) => {
                eprintln!("bad");
                false
            },
            None => {
                eprintln!("validate: {}", signup_data.validate().is_ok());
                signup_data.validate().is_ok()
            }
        }
        
    }

    fn set_username(mut user: WsActor, new_username: String) {
        user.username = new_username;
    }

    // Register a new WebSocket actor
    fn register(&self, id: String, addr: Addr<WsActor>) {
        let mut actor_registry = self.actor_registry.lock().unwrap();
        actor_registry.insert(id, addr);
    }

    // Unregister a WebSocket actor
    fn unregister(&self, id: &str) {
        let mut actor_registry = self.actor_registry.lock().unwrap();
        actor_registry.remove(id);
    }

}

struct WebSocketConnections {
    connections: HashMap<Uuid, Vec<Addr<WsActor>>>,
}

#[derive(Serialize, Deserialize, Clone)]
enum SubscriberState {
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
struct Subscriber {
    user_id: String,
    room_id: String,
    connection_state: SubscriberState,
}

#[derive(Serialize, Deserialize, Clone)]
struct Room {
    name: String,
    room_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct UserMessage {
    unique_id: String,
    content: String,
    sender_id: String,
    room_id: String,
    timestamp: u64,
    message_type: MessageTypes,
}

#[derive(Serialize, Deserialize, Clone)]
enum MessageTypes {
    SetUsername,
    AddToRoom,
    CreateRoom,
    ChangeRoom,
    RemoveFromRoom,
    Basic,
}

#[derive(Serialize, Deserialize, Clone)]
struct UsernameChangedMessage {
    old_username: String,
    new_username: String,
}

#[derive(Serialize, Deserialize)]
struct IncomingMessage {
    content: String,
    username: String,
    message_type: MessageTypes,
}

struct WsMessage(pub String, pub String);

impl actix::Message for WsMessage {
    type Result = ();
}

struct WsActor {
    ws_id: String,
    user_id: String,
    username: String,
    current_room: String,
    rooms: Vec<String>, 
    state: Arc<AppState>, 
}

impl WsActor {
    fn set_username(&mut self, new_username: String) {
        self.username = new_username;
    }

    async fn delete_message(&self, unique_id: String) -> Result<()> {
        let _: Option<UserMessage> = self.state.db.delete(("messages", unique_id))
            .await.expect("error sending_message");

        Ok(())
    }
}


impl Actor for WsActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {


        //registers ws actor
        self.state.actor_registry.lock().unwrap().insert(self.ws_id.clone(), ctx.address());

        let app_state = self.state.clone();
        let room_id = self.current_room.clone();
        let actor_addr = ctx.address();
        let ws_id = self.ws_id.clone();
        let default_username = self.username.clone();
        let room_ids = self.rooms.clone();
        let connections = self.state.db.clone();

        let actor_addr_clone = actor_addr.clone();
        let actor_addr_clone2 = actor_addr.clone();
        let db = self.state.db.clone();


        ctx.spawn(actix::fut::wrap_future(get_users(db, actor_addr_clone2)));

        let init_message = json!({
            "type": "init",
            "uuid": self.user_id,
            "username": default_username,
        });

        ctx.text(init_message.to_string());

        actix::spawn(async move {

            match app_state.catch_up(&room_id).await {
                Ok(messages) => {
                    for message in messages {

                        let serialized_msg = serde_json::to_string(&message).unwrap();
                        
                        actor_addr_clone.do_send(WsMessage(serialized_msg, room_id.clone()));
                    }
                }
                Err(e) => {
                    eprintln!("Error catching up messages: {:?}", e);
                }
            }

        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {

        let user_id = self.username.to_string();
        let room_ids = self.rooms.clone();

        let db = self.state.db.clone();

        actix::spawn(async move {
            for room_id in room_ids {
                //let connection:Option<Connection> = connections.delete(("connections", string_user_id.clone())).await.expect("OOOH");
                let query = format!(
                    "UPDATE subscribers SET connection_state = 'Inactive' 
                    WHERE user_id = '{}' AND room_id = '{}';",
                    user_id, room_id
                );
                eprintln!("{}", query);
                db.query(query).await.expect("something");

            }
        });
    }
}

impl Handler<WsMessage> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        // Always send the message to the client, including the sender
        ctx.text(msg.0);
    }
}

#[derive(Serialize, Deserialize)]
struct UpdateUsernameMsg(String);

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
struct SendUserStruct {
    user_hashmap: HashMap<String, String>,
    message_type : String,
}

#[derive(Serialize, Deserialize)]
struct SendUsers(String);

impl Message for SendUsers {
    type Result = ();
}

impl Handler<SendUsers> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: SendUsers, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0)
        
    }
}

#[derive(Deserialize)]
struct User {
    uuid: String,
    username: String,
}

pub async fn get_users(db: Arc<Surreal<Client>>,  actor_addr: Addr<WsActor>) {

    let sql = r#"SELECT uuid, username FROM users"#;

    let mut response = db.query(sql)
        .await.expect("bad");

    let users: Vec<User> = response.take(0).expect("bad");

    // Convert the vector of tuples into a hash map
    //let users_map: HashMap<String, String> = users.into_iter().collect();
    let users_map: HashMap<String, String> = users.into_iter()
        .map(|user| (user.uuid, user.username))
        .collect();


    let send_user = SendUserStruct {
        user_hashmap: users_map,
        message_type: "user_list".to_string(),
    };

    let serialized = serde_json::to_string(&send_user).unwrap();

    actor_addr.do_send(SendUsers(serialized));

}

/*
    pub async fn get_users(&self) -> Result<Vec<(String, String)>> {

        let sql = r#"SELECT uuid, username FROM users"#;
    
        let mut response = self.db.query(sql)
            .await?;
    
        let users: Vec<(String, String)> = response.take(0)?;
    
        Ok(users)
    }
*/

async fn check_and_update_username(db: Arc<Surreal<Client>>, user_id: String, current_username: String, new_username: String, actor_addr: Addr<WsActor>, state: Arc<AppState>) {
    let query = format!("SELECT username FROM users WHERE username = '{}';", new_username);
    if let Ok(mut response) = db.query(&query).await {
        let x: Option<String> = response.take((0, "username")).expect("nah");

        match x {
            Some(_) => println!("Username already used"),
            None =>  {
                let update_query = format!("UPDATE users SET username = '{}' WHERE username = '{}';", new_username, current_username);
                db.query(&update_query).await.expect("Failed to update database");
                //let message = UpdateUsernameMsg(new_username);
                let message = json!({
                    "type": "update_username",
                    "username": new_username,
                    "sender": current_username,
                    "user_id": user_id,
                });
                let serialized_msg = serde_json::to_string(&message).unwrap();
                state.broadcast_message(serialized_msg, current_username, "nothing".to_string()).await;
                actor_addr.do_send(UpdateUsernameMsg(new_username));
            }
                
        }
    } else {
        eprintln!("Failed to query database");
    }
}

/*
async fn check_and_update_username(db: Arc<Surreal<Client>>, current_username: String, new_username: String, actor_addr: Addr<WsActor>, state: Arc<AppState>) {
    let query = format!("SELECT username FROM users WHERE username = '{}';", new_username);
    if let Ok(mut response) = db.query(&query).await {
        let x: Option<String> = response.take((0, "username")).expect("nah");

        match x {
            Some(_) => println!("Username already used"),
            None =>  {
                let update_query = format!("UPDATE users SET username = '{}' WHERE username = '{}';", new_username, current_username);
                db.query(&update_query).await.expect("Failed to update database");
                let message = UpdateUsernameMsg(new_username);
                let serialized_msg = serde_json::to_string(&message).unwrap();
                state.broadcast_message(serialized_msg, current_username, "nothing".to_string());
                //actor_addr.do_send(UpdateUsernameMsg(new_username))}
                
        }
    } else {
        eprintln!("Failed to query database");
    }
}

*/

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {                
                Ok(incoming_message) => {
                    if incoming_message.username == self.user_id {
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
                                
    
                                actix::spawn(async move {
                                    let _ : Vec<Room> = app_state.db.create("rooms")
                                        .content(Room {
                                            name: room_name,
                                            room_id,
                                        })
                                        .await.expect("bad");
                                });
                            }
                            MessageTypes::AddToRoom => {
                                let room_id = incoming_message.content.clone();
                                let user_id = incoming_message.content;
                                let app_state = self.state.clone();
    
                                actix::spawn(async move {
                                    let query = format!{"UPDATE rooms SET subscribers += ['{}'] WHERE room_id = '{}';", user_id, room_id };
                                    if let Err(e) = app_state.db.query(&query).await {
                                        eprintln!("Error adding to room: {:?}", e);
                                    }
                                });
                            }
                            MessageTypes::ChangeRoom => {
                                let room_id = incoming_message.content;
                                let user_id = self.username.clone();
                                let app_state = self.state.clone();
                                actix::spawn(async move {
                                    let query = format!(
                                        "UPDATE connections SET state = {} WHERE id = '{}'",
                                        user_id, room_id
                                    );
                                    if let Err(e) = app_state.db.query(&query).await {
                                        eprintln!("Error adding to room: {:?}", e);
                                    }
                                });
                            }
                            MessageTypes::RemoveFromRoom => {}
                            MessageTypes::Basic => {
                                let content = incoming_message.content;
            
                                let app_state = self.state.clone();
                                let sender_id = self.user_id.clone();
                                //let sender_id = self.username.clone();
                                let user_id =  self.user_id.clone();
                                let room_id = self.rooms[0].clone();
                                let now = Utc::now();
                                let timestamp = now.timestamp() as u64;
                                let unique_id = Uuid::new_v4().to_string().replace('-', "");
                                // Create the message only once, reusing the variables directly
                                let message = UserMessage {
                                    unique_id: unique_id.clone(),
                                    content: content.clone(),
                                    timestamp,
                                    sender_id: sender_id.clone(),
                                    room_id: room_id.clone(), 
                                    message_type: MessageTypes::Basic,
                                };
                    
                                actix::spawn(async move {
            
                                    let _: Option<UserMessage> = app_state.db.create(("messages", unique_id.clone()))
                                        .content(UserMessage {
                                            unique_id,
                                            content,
                                            timestamp,
                                            sender_id: sender_id.clone(),
                                            room_id: room_id.clone(),
                                            message_type: MessageTypes::Basic,
                                        })
                                        .await.expect("error sending_message");
                    
                                    let serialized_msg = serde_json::to_string(&message).unwrap();
                                    app_state.broadcast_message(serialized_msg, sender_id, room_id).await;
                                });
                                },
                            }
                    } else {
                        println!("No bueno");
                        eprintln!("Invalid access");
                    }
                    
                }

                Err(e) => eprintln!("Error processing message: {:?}", e),
            }
        }
    }
}

/*
impl Handler<WsMessage> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        // Always send the message to the client, including the sender
        ctx.text(msg.0);
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {                
                Ok(incoming_message) => {
                    if incoming_message.username == self.username {
                        match incoming_message.message_type {
                            MessageTypes::SetUsername =>  {
                                let new_username = incoming_message.content;
                                let username = self.username.clone();
                                let state = self.state.clone();
                                let db = self.state.db.clone();
                                let arc_self = Arc::new(self);
                                
                                actix::spawn(async move{
                                    // Use the cloned `username` for querying
                                    let query = format!("SELECT username FROM users WHERE username = '{}';", username);
                                    let mut response = db.query(&query).await.expect("Failed to query database");
    
                                    let username_query: Option<String> = response.take((0, "username")).expect("Failed to extract username");

                                    match username_query {
                                        Some(_) => {
                                            println!("Username already used");
                                        },
                                        None => {
                                            // Use `new_username` and `user_id` for the update
                                            let update_query = format!("UPDATE users SET username = '{}' WHERE username = '{}';", new_username, username);
                                            //arc_self.set_username(new_username);
                                            actix::spawn(async move {
                                                db.query(&update_query).await.expect("Failed to update database");
                                            });
                                        }
                                    }
                                });
                                
                            }
                            MessageTypes::CreateRoom => {
                                let room_id = Uuid::new_v4().to_string().replace('-', "");
                                let room_name = incoming_message.content;
                                let app_state = self.state.clone();
                                
    
                                actix::spawn(async move {
                                    let _ : Vec<Room> = app_state.db.create("rooms")
                                        .content(Room {
                                            name: room_name,
                                            room_id,
                                        })
                                        .await.expect("bad");
                                });
                            }
                            MessageTypes::AddToRoom => {
                                let room_id = incoming_message.content.clone();
                                let user_id = incoming_message.content;
                                let app_state = self.state.clone();
    
                                actix::spawn(async move {
                                    let query = format!{"UPDATE rooms SET subscribers += ['{}'] WHERE room_id = '{}';", user_id, room_id };
                                    if let Err(e) = app_state.db.query(&query).await {
                                        eprintln!("Error adding to room: {:?}", e);
                                    }
                                });
                            }
                            MessageTypes::ChangeRoom => {
                                let room_id = incoming_message.content;
                                let user_id = self.username.clone();
                                let app_state = self.state.clone();
                                actix::spawn(async move {
                                    let query = format!(
                                        "UPDATE connections SET state = {} WHERE id = '{}'",
                                        user_id, room_id
                                    );
                                    if let Err(e) = app_state.db.query(&query).await {
                                        eprintln!("Error adding to room: {:?}", e);
                                    }
                                });
                            }
                            MessageTypes::RemoveFromRoom => {}
                            MessageTypes::Basic => {
                                let content = incoming_message.content;
            
                                let app_state = self.state.clone();
                                let sender_id = self.username.clone();
                                let username =  self.username.clone();
                                let room_id = self.rooms[0].clone();
                                let now = Utc::now();
                                let timestamp = now.timestamp() as u64;
                                let unique_id = Uuid::new_v4().to_string().replace('-', "");
                                // Create the message only once, reusing the variables directly
                                let message = Message {
                                    unique_id: unique_id.clone(),
                                    username: username.clone(), 
                                    content: content.clone(),
                                    timestamp,
                                    sender_id: sender_id.clone(),
                                    room_id: room_id.clone(), 
                                    message_type: MessageTypes::Basic,
                                };
                    
                                actix::spawn(async move {
            
                                    let _: Option<Message> = app_state.db.create(("messages", unique_id.clone()))
                                        .content(Message {
                                            unique_id,
                                            username,
                                            content,
                                            timestamp,
                                            sender_id: sender_id.clone(),
                                            room_id: room_id.clone(),
                                            message_type: MessageTypes::Basic,
                                        })
                                        .await.expect("error sending_message");
                    
                                    let serialized_msg = serde_json::to_string(&message).unwrap();
                                    app_state.broadcast_message(serialized_msg, sender_id, room_id).await;
                                });
                                },
                            }
                    } else {
                        println!("No bueno");
                        eprintln!("Invalid access");
                    }
                    
                }

                Err(e) => eprintln!("Error processing message: {:?}", e),
            }
        }
    }
}

*/

async fn ws_index(req: actix_web::HttpRequest, stream: web::Payload, state: web::Data<AppState>, session: Session) -> std::result::Result<HttpResponse, actix_web::Error> {
    let main_room_id = state.main_room_id.clone();
    let uuid: String = session.get("key").unwrap().unwrap();
    let query = format!{"SELECT username FROM users WHERE uuid = '{}';", uuid};
    let mut response = state.db.query(query).await.expect("aaaah");
    let username_query: Option<String> = response.take((0, "username")).expect("cool");

    match username_query {
        Some(username) => {

            //works it updates connection_state to inactive where etczz
            /*
            let query = format!(
                "UPDATE subscribers SET connection_state = 'Inactive' 
                WHERE user_id = 'test@gmail.com' AND room_id = '{}';",
                main_room_id
            );

            let update_query = format!(
                "UPDATE connections SET subscribers += [{{
                    connection_state: '{}',
                    user_id: '{}'
                }}] WHERE room_id = '{}';",
                SubscriberState::Active, login_username, main_room_id
            );

            state.db.query(query).await.expect("bad");
            */

            let ws_actor = WsActor {
                user_id: uuid,
                ws_id: Uuid::new_v4().to_string().replace('-', ""),
                username,
                current_room: main_room_id.clone(),
                rooms: vec![main_room_id],
                state: state.into_inner().clone(),
            };
        
            ws::start(ws_actor, &req, stream)
        },
        None => {
            
            session.purge();  
            return Ok(HttpResponse::Found().append_header(("LOCATION", "/login")).finish());
        }
    }
}

#[get("/logout")]
async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
}

#[get("/login")]
async fn login_form() -> impl Responder {
    let path = "static/login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read homepage HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Deserialize, Validate)]
struct LoginForm {
    #[validate(email)]
    username: String,
    #[validate(length(min = 1))]
    password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserData {
    uuid: String,
    login_username: String,
    username: String,
    hashed_password: String,
}

async fn create_login_page() -> impl Responder {
    let path = "static/create_login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read homepage HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn create_login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {

    let login = form.into_inner();

    if state.valid_user_credentials(&login).await {
        let mut generator = Generator::with_naming(Name::Numbered);
        let hashed_password = hash(login.password.clone(), DEFAULT_COST).unwrap();
        
        
        let subscriber = Subscriber {
            room_id: state.main_room_id.clone(),
            user_id: login.username.clone(),
            connection_state: SubscriberState::Active,
        };
        let _ : Vec<Subscriber> = state.db.create("subscribers")
            .content(subscriber)
            .await.expect("bad");

        let user_id = Uuid::new_v4().to_string().replace('-', "");
        let _: Vec<UserData> = state.db.create("users").content(
            UserData {
                uuid: user_id.clone(),
                login_username: login.username.clone(),
                username: generator.next().unwrap().replace('-', ""),
                hashed_password,
            }).await.expect("shit");

        session.insert("key", user_id).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
}

async fn login_page() -> impl Responder {
    let path = "static/login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read homepage HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {
    let login = form.into_inner(); // Extracts the LoginForm from the web::Json wrapper

    match state.authenticate_user(&login).await {
        Some(username) => {
            session.insert("key", username).unwrap();
            HttpResponse::Found().append_header(("LOCATION", "/")).finish()
        }
        None => HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
    /*
    if state.authenticate_user(&login).await {
        session.insert("key", login.username.clone()).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
    */
}

#[get("/")]
async fn home_page(session: Session) -> impl Responder {
    let val: Option<String> = session.get("key").unwrap();

    match val {
        //some user_id
        Some(_) => {
            let path = "static/home_page.html";
            match read_to_string(path) {
                Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
                Err(err) => {
                    eprintln!("Failed to read homepage HTML: {:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        None => {
            HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Surreal::new::<Ws>("localhost:8000").await.expect("something_horrible");

    db.signin(Root {
        username: "root",
        password: "root",
    }).await.expect("error");

    db.use_ns("general").use_db("all").await.expect("Something bad");

    let hashed_password = match hash("password", DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(_) => {
            return Ok(())
        }
    };

    let test: Option<UserData> = db.create(("users", "test"))
        .content(UserData {
            uuid: Uuid::new_v4().to_string().replace('-', ""),
            login_username: "test@gmail.com".to_string(),
            username: "test".to_string(),
            hashed_password,

        }).await.expect("hahahah");

    

    let main_room_id = Uuid::new_v4().to_string().replace('-', "");

    let subscriber = Subscriber {
        room_id: main_room_id.clone(),
        user_id: "test@gmail.com".to_string(),
        connection_state: SubscriberState::Active,
    };

    let _ : Vec<Subscriber> = db.create("subscribers")
        .content(subscriber)
        .await.expect("bad");

    let _ : Vec<Room> = db.create("rooms")
        .content(Room {
            name: "main".to_string(),
            room_id: main_room_id.clone(),
        })
        .await.expect("bad");
                            

    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
        channels: Arc::new(Mutex::new(HashMap::new())),
        main_room_id,
        actor_registry: Arc::new(Mutex::new(HashMap::new())),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(home_page)
            .route("/login", web::get().to(login_page))
            .route("/login", web::post().to(login_action))
            .route("/create_login", web::get().to(create_login_page))
            .route("/create_login", web::post().to(create_login_action))
            .service(logout)
            .route("/ws/", web::get().to(ws_index))
            .service(actix_files::Files::new("/static", "static").show_files_listing())
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            
    })
    
    .bind(("127.0.0.1", 8080))?
    //.bind(("192.168.0.155", 8080))?
    .run()
    .await
}
