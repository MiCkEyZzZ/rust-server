extern crate actix_web;

use actix_web::{
    error::{Error, InternalError, JsonPayloadError},
    get, middleware, post, web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Result,
};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

static SERVER_COUNTER: AtomicUsize = AtomicUsize::new(0);
const LOG_FORMAT: &'static str = r#""%r" %s %b "%{User-Agent}i" %D"#;

pub struct RustServerApp {
    port: u16,
}

#[derive(Clone)]
struct AppState {
    server_id: usize,
    request_count: Cell<usize>,
    messages: Arc<Mutex<Vec<String>>>,
}

#[derive(Serialize)]
struct IndexResponse {
    server_id: usize,
    request_count: usize,
    messages: Vec<String>,
}

#[derive(Deserialize)]
struct PostInput {
    message: String,
}

#[derive(Serialize)]
struct PostResponse {
    server_id: usize,
    request_count: usize,
    message: String,
}

#[derive(Serialize)]
struct PostError {
    server_id: usize,
    request_count: usize,
    error: String,
}

#[derive(Serialize)]
struct LookupResponse {
    server_id: usize,
    request_count: usize,
    result: Option<String>,
}

#[get("/")]
async fn index(state: web::Data<AppState>) -> Result<web::Json<IndexResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let ms = state.messages.lock().unwrap();

    Ok(web::Json(IndexResponse {
        server_id: state.server_id,
        request_count,
        messages: ms.clone(),
    }))
}

async fn post(
    msg: web::Json<PostInput>,
    state: web::Data<AppState>,
) -> Result<web::Json<PostResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let mut ms = state.messages.lock().unwrap();
    ms.push(msg.message.clone());

    Ok(web::Json(PostResponse {
        server_id: state.server_id,
        request_count,
        message: msg.message.clone(),
    }))
}

#[post("/clear")]
async fn clear(state: web::Data<AppState>) -> Result<web::Json<IndexResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let mut ms = state.messages.lock().unwrap();
    ms.clear();

    Ok(web::Json(IndexResponse {
        server_id: state.server_id,
        request_count,
        messages: vec![],
    }))
}

fn post_error(err: JsonPayloadError, req: &HttpRequest) -> Error {
    let extns = req.extensions();
    let state = extns.get::<web::Data<AppState>>().unwrap();
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let post_error = PostError {
        server_id: state.server_id,
        request_count,
        error: format!("{}", err),
    };
    InternalError::from_response(err, HttpResponse::BadRequest().json(post_error)).into()
}

#[get("/lookup/{index}")]
async fn lookup(
    state: web::Data<AppState>,
    idx: web::Path<usize>,
) -> Result<web::Json<LookupResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let ms = state.messages.lock().unwrap();
    let result = ms.get(idx.into_inner()).cloned();

    Ok(web::Json(LookupResponse {
        server_id: state.server_id,
        request_count,
        result,
    }))
}

impl RustServerApp {
    pub fn new(port: u16) -> Self {
        RustServerApp { port }
    }

    #[actix_web::main]
    pub async fn run(&self) -> std::io::Result<()> {
        let messages = Arc::new(Mutex::new(vec![]));
        println!("Server listening on port {}...", self.port);
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(AppState {
                    server_id: SERVER_COUNTER.fetch_add(1, Ordering::SeqCst),

                    request_count: Cell::new(0),
                    messages: messages.clone(),
                }))
                .wrap(middleware::Logger::new(LOG_FORMAT))
                .service(index)
                .service(
                    web::resource("/send")
                        .app_data(
                            web::JsonConfig::default()
                                .limit(4096)
                                .error_handler(post_error),
                        )
                        .route(web::post().to(post)),
                )
                .service(clear)
                .service(lookup)
        })
        .bind(("127.0.0.1", self.port))?
        .workers(8)
        .run()
        .await
    }
}
