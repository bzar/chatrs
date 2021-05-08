use actix_files::Files;
use actix_web::{middleware, App, HttpServer};
use std::env::current_exe;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let static_path = {
        let mut exe_path = current_exe()?;
        exe_path.pop();
        exe_path.push("web_client");
        exe_path
    };
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/", &static_path).index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .workers(1)
    .run()
    .await
}
