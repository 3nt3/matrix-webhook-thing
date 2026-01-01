use actix_web::{App, HttpServer, get};
use log::{debug, info};
use matrix_sdk::{
    Client, ServerName,
    config::SyncSettings,
    debug,
    ruma::{
        __private_macros::server_name,
        RoomId,
        events::room::message::{RoomMessageEventContent, SyncRoomMessageEvent},
        user_id,
    },
};

static MATRIX_CLIENT: once_cell::sync::OnceCell<Client> = once_cell::sync::OnceCell::new();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let homeserver_url =
        std::env::var("MATRIX_HOMESERVER_URL").unwrap_or_else(|_| "https://3nt3.de".to_string());

    let beepboop = user_id!("@beepboop:3nt3.de");
    let client = Client::builder()
        .homeserver_url(homeserver_url)
        .build()
        .await
        .expect("Failed to create client");

    client
        .matrix_auth()
        .login_username(
            beepboop.localpart(),
            std::env::var("MATRIX_PASSWORD")
                .expect("MATRIX_PASSWORD not set")
                .as_str(),
        )
        .send()
        .await
        .expect("Login failed");

    client.sync_once(SyncSettings::default()).await.unwrap();

    MATRIX_CLIENT.set(client).unwrap();

    info!("Logged in as {}", beepboop);

    HttpServer::new(|| App::new().service(new_commit))
        .bind(("127.0.0.1", 1337))?
        .run()
        .await
}

#[get("/new-commit")]
async fn new_commit() -> impl actix_web::Responder {
    info!("New commit endpoint hit");

    let client = MATRIX_CLIENT.get().unwrap();

    let room_id = RoomId::parse(
        std::env::var("MATRIX_ROOM_ID").unwrap_or_else(|_| "!open-vaporphase:3nt3.de".to_string()),
    )
    .unwrap();

    debug!("Sending message to room {}", room_id);

    let room = client.get_room(&room_id).expect("Room not found");

    let msg = RoomMessageEventContent::text_plain("New commit pushed to repository!");
    room.send(msg).await.unwrap();

    "New commit received"
}
