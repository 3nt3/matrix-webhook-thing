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
use serde::{self, Deserialize};

#[derive(Debug, Deserialize)]
pub struct GithubWebhook {
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub before: String,
    pub after: String,
    pub repository: Repository,
    pub commits: Vec<Commit>,
    pub head_commit: Commit,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub timestamp: String,
    pub url: String,
    pub author: CommitUser,
}

#[derive(Debug, Deserialize)]
pub struct CommitUser {
    pub name: String,
    pub email: String,
    pub username: Option<String>,
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

    HttpServer::new(|| App::new().service(new_commit).service(hello))
        .bind(("0.0.0.0", 1337))?
        .run()
        .await
}

#[get("/")]
async fn hello() -> impl actix_web::Responder {
    "Hello, world!"
}

#[post("/git/new-commit")]
async fn new_commit(payload: Json<GithubWebhook>) -> impl actix_web::Responder {
    info!("New commit endpoint hit");

    let client = MATRIX_CLIENT.get().unwrap();

    let room_id = RoomId::parse(std::env::var("MATRIX_ROOM_ID").unwrap()).unwrap();

    debug!("Sending message to room {}", room_id);

    let room_for_repo =
        get_room_for_repo(&payload.repository.full_name).expect("Failed to get room for repo");
    let room_id = RoomId::parse(room_for_repo).expect("Invalid room ID");
    let room = client.get_room(&room_id).expect("Failed to get room");

    let users = format_list(
        payload
            .commits
            .iter()
            .map(|c| c.author.username.clone().unwrap_or(c.author.name.clone()))
            .collect::<Vec<String>>(),
    );

    let msg = RoomMessageEventContent::text_html(
        format!(
            "New commit by {}: \"{}\" ",
            users, payload.commits[0].message
        ),
        format!(
            "<a href=\"{}\">New commit by {}</a><br><blockquote>{}</blockquote>",
            payload.commits[0].url, users, payload.commits[0].message,
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

fn format_list(items: Vec<String>) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let last = items.last().unwrap();
            let rest = &items[..items.len() - 1];
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}
