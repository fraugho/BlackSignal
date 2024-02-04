use actix_web::{get, post, web, App, HttpServer, HttpResponse, Responder};
use actix_session::{Session, CookieSession};
use actix_web_lab::extract::Path;
use actix_files::Files;

use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::fs::read_to_string;
use std::io::Write;

//local packages
mod websocket;
mod appstate;
mod structs;
mod message_structs;

use structs::{Room, ConnectionState, LoginForm, UserData};
use message_structs::*;

use local_ip_address::local_ip;
use websocket::ws_index;
use appstate::AppState;

use serde::{Deserialize, Serialize};

use bcrypt::{hash, DEFAULT_COST};

use names::{Generator, Name};

use serde_json::json;

use uuid::Uuid;



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
            }
        ).await.expect("Failed to create user");
        state.join_main_room(username.clone(), user_id.clone()).await;

        let query = "UPDATE rooms SET users += $user_id WHERE room_id = $room_id;";
        if let Err(e) = state.db.query(query).bind(("user_id", user_id.clone())).bind(("room_id", state.main_room_id.clone())).await {
            eprintln!("Error adding to room: {:?}", e);
        }

        let message = UserMessage::NewUser(NewUserMessage::new(user_id.clone(), username.clone()));
        let serilized_message = serde_json::to_string(&message).unwrap();

        state.broadcast_message(serilized_message, state.main_room_id.clone(), user_id.clone()).await;
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
    let login = form.into_inner();
    match state.authenticate_user(&login).await {
        Some(username) => {
            if session.insert("key", username).is_ok() {
                HttpResponse::Found().append_header(("LOCATION", "/")).finish()
            } else {
                HttpResponse::Found().append_header(("LOCATION", "/login")).finish()
            }
        }
        None => HttpResponse::Ok().json(json!({"success": false, "message": "Invalid credentials"}))
    }
}

#[derive(Serialize, Deserialize)]
struct Image {
    filename: String,
    data: Vec<u8>,
}

#[post("/upload")]
async fn upload(upload: web::Json<Image>, session: Session) -> impl Responder {
    // Access the uploaded image data from the request body
    let image_data = upload.into_inner();

    // Handle the image upload logic here
    // You can save the image to a file, store it in a database, or perform any other necessary operations

    // Example: Save the image to the "/Images" directory
    let file_name = Uuid::new_v4().to_string().replace('-', "");
    //let image_filename = format!("/Images/{}.jpg", image_data.filename); // Assuming the filename is provided in the Image struct
    let image_filename = format!("/Images/{}.jpg", file_name);
    let image_path = std::path::Path::new(&image_filename);
    let mut image_file = std::fs::File::create(image_path).expect("Failed to create image file");
    image_file.write_all(&image_data.data).expect("Failed to write image data to file");

    // Example response
    HttpResponse::Ok()
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


    //creates test user
    let _: Option<UserData> = db.create(("users", "test"))
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