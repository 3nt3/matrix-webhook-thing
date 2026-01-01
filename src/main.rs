use actix_web::{App, HttpServer, get};
use log::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    HttpServer::new(|| App::new().service(new_commit))
        .bind(("127.0.0.1", 1337))?
        .run()
        .await
}

#[get("/new-commit")]
async fn new_commit() -> impl actix_web::Responder {
    info!("New commit endpoint hit");

    "New commit received"
}
