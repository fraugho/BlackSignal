use actix_web::{get, post, web, App, HttpServer, HttpResponse, Responder};
use actix_session::{Session, CookieSession};

use names::{Generator, Name};

use bcrypt::{hash, DEFAULT_COST};

use surrealdb::Surreal;
use surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::Ws;

use uuid::Uuid;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs::read_to_string;

use serde_json::json;

mod appstate;
mod structs;
mod websocket;

use appstate::AppState;
use structs::{Room, ConnectionState, LoginForm, UserData};
use websocket::ws_index;
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
    let path = "static/create_login_page.html";
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
                uuid: user_id.clone(),
                login_username: login.username.clone(),
                username: username.clone(),
                hashed_password,
                status: ConnectionState::Online,
                rooms: vec![state.main_room_id.clone()],
            }).await.expect("shit");

        state.join_main_room(username, user_id.clone()).await;

        let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
        if let Err(e) = state.db.query(query).bind(("user_id", user_id.clone())).bind(("room_id", state.main_room_id.clone())).await {
            eprintln!("Error adding to room: {:?}", e);
        }

        session.insert("key", user_id).unwrap();
        HttpResponse::Found().append_header(("LOCATION", "/")).finish()
    } else {
        HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
}

#[get("/login")]
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

#[post("/login")]
async fn login_action(state: web::Data<AppState>, form: web::Json<LoginForm>, session: Session) -> impl Responder {
    let login = form.into_inner(); // Extracts the LoginForm from the web::Json wrapper

    match state.authenticate_user(&login).await {
        Some(username) => {
            session.insert("key", username).unwrap();
            HttpResponse::Found().append_header(("LOCATION", "/")).finish()
        }
        None => HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
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


    let main_room_id = Uuid::new_v4().to_string().replace('-', "");
    let uuid = Uuid::new_v4().to_string().replace('-', "");
    let test: Option<UserData> = db.create(("users", "test"))
        .content(UserData {
            uuid: uuid.clone(),
            login_username: "test@gmail.com".to_string(),
            username: "test".to_string(),
            hashed_password,
            status: ConnectionState::Online,
            rooms: vec![main_room_id.clone()],

        }).await.expect("hahahah");
    

    

    let users = vec![uuid];
    let _ : Vec<Room> = db.create("rooms")
        .content(Room {
            name: "main".to_string(),
            room_id: main_room_id.clone(),
            users,
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
            .service(login_page)
            .service(login_action)
            .service(create_login_page)
            .service(create_login_action)
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
