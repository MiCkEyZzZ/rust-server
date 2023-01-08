extern crate actix_web;

use actix_web::{get, middleware, web, App, HttpRequest, HttpServer, Result};
use serde::Serialize;

#[derive(Serialize)]
struct IndexResponse {
    message: String,
}

#[get("/")]
async fn index(req: HttpRequest) -> Result<web::Json<IndexResponse>> {
    let hello = req
        .headers()
        .get("hello")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "world");

    Ok(web::Json(IndexResponse {
        message: hello.to_owned(),
    }))
}

pub struct RustServerApp {
    port: u16,
}

impl RustServerApp {
    pub fn new(port: u16) -> Self {
        RustServerApp { port }
    }

    #[actix_web::main]
    pub async fn run(&self) -> std::io::Result<()> {
        println!("Server listening on port {}...", self.port);
        HttpServer::new(move || {
            App::new()
                .wrap(middleware::Logger::default())
                .service(index)
        })
        .bind(("127.0.0.1", self.port))?
        .workers(8)
        .run()
        .await
    }
}