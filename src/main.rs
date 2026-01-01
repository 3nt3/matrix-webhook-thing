use actix_web::{App, HttpServer, get, post, web::Json};
use anyhow::anyhow;
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

#[derive(serde::Deserialize)]
struct PushPayload {
    repository: String,
    sender: String,
    commits: Vec<Commit>,
}

#[derive(serde::Deserialize)]
struct Commit {
    author: String,
    url: String,
    message: String,
}

#[derive(serde::Deserialize)]
struct Config {
    repos: Vec<Repo>,
}

#[derive(serde::Deserialize)]
struct Repo {
    id: String,
    room: String,
}

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

#[post("/git/new-commit")]
async fn new_commit(payload: Json<PushPayload>) -> impl actix_web::Responder {
    info!("New commit endpoint hit");

    let client = MATRIX_CLIENT.get().unwrap();

    let room_id = RoomId::parse(
        std::env::var("MATRIX_ROOM_ID").unwrap_or_else(|_| "!open-vaporphase:3nt3.de".to_string()),
    )
    .unwrap();

    debug!("Sending message to room {}", room_id);

    let room_for_repo =
        get_room_for_repo(&payload.repository).expect("Failed to get room for repo");
    let room_id = RoomId::parse(room_for_repo).expect("Invalid room ID");
    let room = client.get_room(&room_id).expect("Failed to get room");

    let msg = RoomMessageEventContent::text_html(
        format!(
            "New commit by {}: \"{}\" ",
            payload.sender, payload.commits[0].message
        ),
        format!(
            "<a href=\"{}\">New commit by {}: <i>{}</i></a><br><blockquote>{}</blockquote>",
            payload.commits[0].url,
            payload.sender,
            payload.commits[0]
                .message
                .lines()
                .next()
                .unwrap_or("No commit message"),
            payload.commits[0].message,
        ),
    );
    room.send(msg).await.unwrap();

    "New commit received"
}

fn get_room_for_repo(repo: &str) -> anyhow::Result<String> {
    let toml_str = std::fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&toml_str)?;

    let room = &config
        .repos
        .iter()
        .find(|x| x.id == repo.to_string())
        .ok_or_else(|| anyhow!("Room not configured"))?
        .room;

    Ok(room.to_string())
}
