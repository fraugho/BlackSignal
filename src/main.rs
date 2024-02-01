use actix_web::{get, post, web, App, HttpServer, HttpResponse, Responder};
use actix_session::{Session, CookieSession};
use actix_web::cookie::Key;
use actix_files::Files;

use names::{Generator, Name};

use bcrypt::{hash, DEFAULT_COST};

use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::Ws;

use uuid::Uuid;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::fs::read_to_string;

use serde_json::json;

mod appstate;
mod structs;
mod websocket;

use appstate::AppState;
use structs::{Room, ConnectionState, LoginForm, UserData};
use websocket::ws_index;

use local_ip_address::local_ip;
//use anyhow::Result;

/*
Tables in DB
Users
Connections
Messages
*/



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
            eprintln!("Failed to read homepage HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/create_login")]
async fn create_login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {

    let login = form.into_inner();

    if state.valid_user_credentials(&login).await {
        let mut generator = Generator::with_naming(Name::Numbered);
        let hashed_password = hash(login.password.clone(), DEFAULT_COST).unwrap();
        

        let user_id = Uuid::new_v4().to_string().replace('-', "");
        let username = generator.next().unwrap().replace('-', "");
        let _: Vec<UserData> = state.db.create("users").content(
            UserData {
                user_id: user_id.clone(),
                login_username: login.username.clone(),
                username: username.clone(),
                hashed_password,
                status: ConnectionState::Online,
                rooms: vec![state.main_room_id.clone()],
            }).await.expect("Failed to create user");

        state.join_main_room(username.clone(), user_id.clone()).await;
        
        //let query = "UPDATE rooms SET users += [$user_id] WHERE room_id = $room_id;", person, room_id };
        let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
        if let Err(e) = state.db.query(query).bind(("user_id", user_id.clone())).bind(("room_id", state.main_room_id.clone())).await {
            eprintln!("Error adding to room: {:?}", e);
        }

        let message = json!({
            "type": "new_user_joined",
            "user_id": user_id,
            "username": username,
        });

        state.broadcast_message(message.to_string(), state.main_room_id.clone(), user_id.clone()).await;

        session.insert("key", user_id).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
}

#[get("/login")]
async fn login_page() -> impl Responder {
    let path = "static/HTML/login_page.html";
    match read_to_string(path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(err) => {
            eprintln!("Failed to read homepage HTML: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/login")]
async fn login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {
    let login = form.into_inner(); // Extracts the LoginForm from the web::Json wrapper

    match state.authenticate_user(&login).await {
        Some(username) => {
            if session.insert("key", username).is_ok(){
                HttpResponse::Found().append_header(("LOCATION", "/")).finish()
            } else {
                HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
            }
            
        }
        None => HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
}

#[derive(Serialize, Deserialize)]
struct Image{

}

#[post("/upload")]
async fn upload(upload: web::Json<Image>, state: web::Data<AppState>, session: Session) -> impl Responder {
    if let Ok(key) = session.get::<String>("key"){
        match key{
            Some(user) => println!("cool"),
            None  => println!("not cool"),
        }
    };
    HttpResponse::Ok()
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
        //some user_id
        Some(_) => {
            let path = "static/HTML/home_page.html";
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
    let db = Surreal::new::<Ws>("localhost:8000").await.expect("unable to connect ot db");

    db.signin(Root {
        username: "root",
        password: "root",
    }).await.expect("Unable to Login to DB");

    db.use_ns("general").use_db("all").await.expect("Not able to use ns of db");

    let hashed_password = match hash("password", DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(_) => {
            return Ok(())
        }
    };


    let main_room_id = Uuid::new_v4().to_string().replace('-', "");
    let user_id = Uuid::new_v4().to_string().replace('-', "");
    let test: Option<UserData> = db.create(("users", "test"))
        .content(UserData {
            user_id: user_id.clone(),
            login_username: "test@gmail.com".to_string(),
            username: "test".to_string(),
            hashed_password,
            status: ConnectionState::Online,
            rooms: vec![main_room_id.clone()],

        }).await.expect("Failed making test user data");
    

    let mut users = HashSet::new();
    users.insert(user_id.clone());
    
    //let users = vec![user_id];
    let _ : Vec<Room> = db.create("rooms")
        .content(Room {
            name: "main".to_string(),
            room_id: main_room_id.clone(),
            users,
        })
        .await.expect("Failed to created test room");


    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
        channels: Arc::new(Mutex::new(HashMap::new())),
        main_room_id,
        actor_registry: Arc::new(Mutex::new(HashMap::new())),
        //room_registry: Arc::new(Mutex::new(room_registry)),
    });

    let my_local_ip = local_ip();
    let address;
    if let Ok(my_local_ip) = my_local_ip {
        address = my_local_ip;
        println!("Go to {}:8080", address);
    } else {
        return Ok(())
    }

    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(home_page)
            .service(login_page)
            .service(login_action)
            .service(create_login_page)
            .service(create_login_action)
            .service(logout)
            .service(get_ip)
            .service(Files::new("/Images", "./Images"))
            .route("/ws/", web::get().to(ws_index))
            .service(actix_files::Files::new("/static", "static").show_files_listing())
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
    })
    
    .bind((address, 8080))?
    //.bind(("192.168.0.155", 8080))?
    .run()
    .await
}
