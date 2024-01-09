use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use actix_web_actors::ws;
use actix_session::{Session, CookieSession};

use names::{Generator, Name};

use bcrypt::{hash, DEFAULT_COST};

use surrealdb::{Result, Surreal};
use surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::{Client, Ws};

use serde::{Serialize, Deserialize};

use uuid::Uuid;

use chrono::Utc;

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

    pub async fn catch_up(&self, room_id: &str) -> Result<Vec<Message>> {

        let sql = r#"SELECT * FROM messages ORDER BY timestamp ASC"#;
    
        let mut response = self.db.query(sql)
            .await?;
    
        let messages: Vec<Message> = response.take(0)?;
    
        Ok(messages)
    }  

    pub async fn get_rooms(&self, user_id: &str) -> Result<Vec<String>> {
        let sql = format! {"SELECT room_id FROM connections WHERE id = '{}'", user_id};
    
        let mut response = self.db.query(sql)
            .await?;
    
        let rooms: Vec<String> = response.take(0)?;
    
        Ok(rooms)
    }

    async fn authenticate_user(&self, login_data: &LoginForm) -> bool{
        let query = format!{"SELECT * FROM users WHERE login_username = '{}';", login_data.username};
        let mut response = self.db.query(query).await.expect("aaaah");
        let result: Option<UserData> = response.take(0).expect("cool");
        //let result: Option<UserData> = self.db.select(("logins", username)).await.expect("something");
        match result {
            Some(user_data) => {
                bcrypt::verify(login_data.password.clone(), &user_data.hashed_password).unwrap_or(false)
            },
            None => {
                false
            }
        }
    }

    async fn valid_user_credentials(&self, signup_data: &LoginForm) -> bool{
        let result: Option<UserData> = self.db.select(("logins", &signup_data.username)).await.expect("something");
        match result {
            Some(_) => {
                println!("shit");
                false
            },
            None => {
                println!("validate: {}", signup_data.validate().is_ok());
                signup_data.validate().is_ok()
            }
        }
        
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
enum ConnectionState {
    Active,
    Inactive,
    Offline,
}

#[derive(Serialize, Deserialize, Clone)]
struct Connection{
    user_id: String,
    connection_state: ConnectionState,
}

#[derive(Serialize, Deserialize, Clone)]
struct Room {
    name: String,
    room_id: String,
    subscribers: Vec<(Connection)>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    unique_id: String,
    username: String,
    content: String,
    sender_id: String,
    room_id: String,
    timestamp: u64,
    messsage_type: MessageTypes,
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

#[derive(Serialize, Deserialize)]
struct IncomingMessage {
    content: String,
    message_type: MessageTypes,
}

struct WsMessage(pub String, pub String);

impl actix::Message for WsMessage {
    type Result = ();
}

struct WsActor {
    ws_id: String,
    login_username: String,
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
        let _: Option<Message> = self.state.db.delete(("messages", unique_id))
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
        let login_username = self.login_username.clone();
        let default_username = self.username.clone();
        let room_ids = self.rooms.clone();
        let connections = self.state.db.clone();

        let actor_addr_clone = actor_addr.clone();

        let init_message = json!({
            "type": "init",
            "user_id": login_username.clone(),
            "username": default_username
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

        let string_user_id = self.ws_id.to_string();
        let room_ids = self.rooms.clone();
        let string_room_ids: Vec<String> = room_ids.iter().map(|x| x.to_string()).collect();
        let connections = self.state.db.clone();

        actix::spawn(async move {
            for room_id in string_room_ids {
                let connection:Option<Connection> = connections.delete(("connections", string_user_id.clone())).await.expect("OOOH");
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

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {                
                Ok(incoming_message) => {
                    match incoming_message.message_type {
                        MessageTypes::SetUsername =>  {
                            let username = incoming_message.content;
                            self.username = username.clone();
                            let query = format! {"UPDATE users SET username = '{}' WHERE login_username = '{}';", username, self.login_username.clone()};
                            let db = self.state.db.clone();
                            actix::spawn(async move{
                                db.query(query.clone()).await.expect("shit");
                            });
                        }
                        MessageTypes::CreateRoom => {
                            let room_id = Uuid::new_v4().to_string().replace('-', "");
                            let room_name = incoming_message.content;
                            let app_state = self.state.clone();
                            let connection =  Connection {
                                user_id: self.login_username.clone(),
                                connection_state: ConnectionState::Active,
                            };

                            actix::spawn(async move {
                                let _ : Vec<Room> = app_state.db.create("connection")
                                    .content(Room {
                                        name: room_name,
                                        room_id,
                                        subscribers: vec![(connection)]
                                    })
                                    .await.expect("bad");
                            });
                        }
                        MessageTypes::AddToRoom => {
                            let room_id = incoming_message.content.clone();
                            let user_id = incoming_message.content;
                            let app_state = self.state.clone();

                            actix::spawn(async move {
                                let query = format!{"UPDATE connections SET subscribers += ['{}'] WHERE room_id = '{}';", user_id, room_id };
                                if let Err(e) = app_state.db.query(&query).await {
                                    eprintln!("Error adding to room: {:?}", e);
                                }
                            });
                        }
                        MessageTypes::ChangeRoom => {
                            let room_id = incoming_message.content;
                            let user_id = self.login_username.clone();
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
                            let sender_id = self.login_username.clone();
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
                                messsage_type: MessageTypes::Basic,
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
                                        messsage_type: MessageTypes::Basic,
                                    })
                                    .await.expect("error sending_message");
                
                                let serialized_msg = serde_json::to_string(&message).unwrap();
                                app_state.broadcast_message(serialized_msg, sender_id, room_id).await;
                            });
                            },
                        }
                    }

                Err(e) => eprintln!("Error processing message: {:?}", e),
            }
        }
    }
}

async fn ws_index(req: actix_web::HttpRequest, stream: web::Payload, state: web::Data<AppState>, session: Session) -> std::result::Result<HttpResponse, actix_web::Error> {
    let main_room_id = state.main_room_id.clone();
    let login_username: String = session.get("key").unwrap().unwrap();
    let query = format!{"SELECT username FROM users WHERE login_username = '{}';", login_username};
    let mut response = state.db.query(query).await.expect("aaaah");
    let username_query: Option<String> = response.take((0, "username")).expect("cool");
    match username_query {
        Some(username) => {
            let ws_actor = WsActor {
                login_username,
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
        let _: Vec<UserData> = state.db.create("users").content(
            UserData {
                login_username: login.username.clone(),
                username: generator.next().unwrap().replace('-', ""),
                hashed_password,
            }).await.expect("shit");

        session.insert("key", login.username.clone()).unwrap();
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

    if state.authenticate_user(&login).await {
        session.insert("key", login.username.clone()).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
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
            login_username: "test@gmail.com".to_string(),
            username: "test".to_string(),
            hashed_password,

        }).await.expect("hahahah");

    let main_room_id = Uuid::new_v4().to_string().replace('-', "");
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
    .bind(("192.168.0.155", 8080))?
    .run()
    .await
}
