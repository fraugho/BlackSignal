use actix_web::{get, post, web, App, HttpServer, HttpResponse, Responder};
use actix_session::{Session, CookieSession};
use actix_files::Files;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::fs::read_to_string;
use std::io::Write;
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use bcrypt::{hash, DEFAULT_COST};
use names::{Generator, Name};
use serde_json::json;
use uuid::Uuid;

// Local packages
mod websocket;
mod appstate;
mod structs;
mod message_structs;

use structs::{Room, ConnectionState, LoginForm, UserData};
use message_structs::*;
use websocket::*;
use appstate::AppState;

#[get("/logout")]
async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
}

#[get("/create_login")]
async fn create_login_page() -> impl Responder {
    let path = "static/HTML/create_login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read create_login_page HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/create_login")]
async fn create_login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {
    let login = form.into_inner();
    if state.valid_user_credentials(&login).await {
        let mut generator = Generator::with_naming(Name::Numbered);

        let user_data = UserData{
            user_id: Uuid::new_v4().to_string().replace('-', ""),
            hashed_password: hash(login.password.clone(), DEFAULT_COST).unwrap(),
            login_username: login.username,
            username: generator.next().unwrap().replace('-', ""),
            status: ConnectionState::Online,
            rooms: vec![state.main_room_id.clone()],

        };
        let _: Vec<UserData> = match state.db.create("users").content(user_data.clone())
            .await {
            Ok(created) => created,
            Err(e) => {log::error!("Failed to get user data: fn create_login_action, error: {:?}", e);
            return HttpResponse::InternalServerError().body("Internal server error: Failed to create user data.")}
        };

        let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
        if let Err(e) = state.db.query(query).bind(("user_id", user_data.user_id.clone())).bind(("room_id", state.main_room_id.clone())).await {
            log::error!("Error adding to room: {:?}", e);
            return HttpResponse::InternalServerError().body("Internal server error: Failed to add user to room in db: fn create_login_action")
        }

        let message = UserMessage::NewUser(NewUserMessage::new(user_data.user_id.clone(), user_data.username));
        let serialized_message = serde_json::to_string(&message).unwrap();

        state.broadcast_message(serialized_message, state.main_room_id.clone(), user_data.user_id.clone()).await;
        session.insert("key", user_data.user_id).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!(LoginErrorMessage::new("Invalid Please enter an email and a password".to_string())))
    }
}

#[get("/login")]
async fn login_page() -> impl Responder {
    let path = "static/HTML/login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read login_page HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/login")]
async fn login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {
    let login = form.into_inner();
    match state.authenticate_user(&login).await {
        Some(username) => {
            if session.insert("key", username).is_ok() {
                HttpResponse::Found().append_header(("LOCATION", "/")).finish()
            } else {
                HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
            }
        }
        None => {
            HttpResponse::Ok().json(json!(LoginErrorMessage::new("Invalid Please enter an email and a password".to_string())))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Image {
    filename: String,
    data: Vec<u8>,
}

#[post("/upload")]
async fn upload(upload: web::Json<Image>) -> impl Responder {
    let image_data = upload.into_inner();
    let file_name = Uuid::new_v4().to_string().replace('-', "");
    let image_filename = format!("/Images/{}.jpg", file_name);
    let image_path = std::path::Path::new(&image_filename);
    let mut image_file = std::fs::File::create(image_path).expect("Failed to create image file");
    image_file.write_all(&image_data.data).expect("Failed to write image data to file");
    HttpResponse::Ok()
}

#[post("/change_username")]
async fn change_username(username_change: web::Json<UserMessage>, session: Session, state: web::Data<AppState>) -> impl Responder {
    let arc_state: Arc<AppState> = state.clone().into_inner();
    if let UserMessage::UsernameChange(message) = username_change.into_inner() { 
        let user_id = match session.get::<String>("key") {
            Ok(Some(id)) => id,
            _ => return HttpResponse::BadRequest().json(json!({"error": "Failed to get user_id from session"})),
        };
        let query = "SELECT * FROM users WHERE user_id = $user_id;";
        if let Ok(mut response) = state.db.query(query).bind(("user_id", user_id.clone())).await{
            let user_query: Option<UserData> = match response.take(0) {
                Ok(data) => data,
                Err(e) => {
                    log::error!("Failed to get user data: fn change_username, error: {:?}", e);
                    return HttpResponse::BadRequest().json(json!({"error": "Failed to get user_id from session"}));
                }
            };
            
            let user_data = user_query.unwrap();
            match check_and_update_username(user_id, user_data.username, message.new_username.clone(), arc_state, UserMessage::UsernameChange(message))
                .await {
                Ok(response) => response,
                Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
            }
        } else {
            HttpResponse::BadRequest().json(json!({"error": "Database Error"}))
        }
    } else {
        HttpResponse::BadRequest().json(json!({"error": "Invalid message format for username change."}))
    }
}

#[get("/get-image/{filename}")]
async fn get_image(path: web::Path<(String,)>) -> impl Responder {
    let filename = &path.0;
    let image_path = format!("./Images/{}", filename);
    match std::fs::read(image_path) {
        Ok(data) => HttpResponse::Ok().content_type("image/jpeg").body(data),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/get-ip")]
async fn get_ip() -> impl Responder {
    match local_ip() {
        Ok(ip) => HttpResponse::Ok().json(json!({"ip": ip.to_string()})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/")]
async fn home_page(session: Session) -> impl Responder {
    let val: Option<String> = session.get("key").unwrap();
    match val {
        Some(_) => {
            let path = "static/HTML/home_page.html";
            match read_to_string(path) {
                Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
                Err(err) => {
                    eprintln!("Failed to read home_page HTML: {:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        None => HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = match Surreal::new::<Ws>("localhost:8000").await {
        Ok(connected) => connected,
        Err(e) => {log::error!("Failed to connect to database: fn main, error: {:?}", e);
        return Ok(())}
    };
    match db.signin(Root {
        username: "root",
        password: "root",
    }).await {
        Ok(connected) => connected,
        Err(e) => {log::error!("Failed to login to database: fn main, error: {:?}", e);
        return Ok(())}
    };
    match db.use_ns("general").use_db("all")
        .await {
        Ok(connected) => connected,
        Err(e) => {log::error!("Failed use namespace of database: fn main, error: {:?}", e);
        return Ok(())}
    };

    let hashed_password = match hash("password", DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(_) => return Ok(())
    };

    let main_room_id = Uuid::new_v4().to_string().replace('-', "");
    let user_id = Uuid::new_v4().to_string().replace('-', "");

    // Create test user
    let _: Option<UserData> = match db.create(("users", "test"))
        .content(UserData {
            user_id: user_id.clone(),
            login_username: "test@gmail.com".to_string(),
            username: "test".to_string(),
            hashed_password,
            status: ConnectionState::Online,
            rooms: vec![main_room_id.clone()],
        }).await {
            Ok(created) => created,
            Err(e) => {log::error!("Failed to create test user data: fn main, error: {:?}", e);
            return Ok(())}
        };

    let mut users = HashSet::new();
    users.insert(user_id);

    let _ : Vec<Room> = match db.create("rooms")
        .content(Room {
            name: "main".to_string(),
            room_id: main_room_id.clone(),
            users,
        })
        .await {
            Ok(created) => created,
            Err(e) => {log::error!("Failed to create room data: fn main, error: {:?}", e);
            return Ok(())}
        };

    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
        channels: Arc::new(Mutex::new(HashMap::new())),
        main_room_id,
        actor_registry: Arc::new(Mutex::new(HashMap::new())),
    });

    let my_local_ip = local_ip();
    let address;

    if let Ok(my_local_ip) = my_local_ip {
        address = my_local_ip;
        println!("Go to {}:8080", address);
    } else {
        return Ok(())
    }

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(home_page)
            .service(login_page)
            .service(login_action)
            .service(create_login_page)
            .service(create_login_action)
            .service(logout)
            .service(change_username)
            .service(get_ip)
            .service(Files::new("/Images", "./Images"))
            .service(get_image)
            .route("/ws/", web::get().to(ws_index))
            .service(actix_files::Files::new("/static", "static").show_files_listing())
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
    })
    .bind((address, 8080))?
    .run()
    .await
}