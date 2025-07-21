mod handlers;
mod models;
mod routes;
mod state;

use crate::routes::task_routes;
use crate::state::AppState;
use actix_web::{App, HttpServer};
use chrono::Local;
use log::info;
use std::error::Error;
use std::io;
use std::io::Write;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日志
    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    log_builder
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S %:z"),
                record.level(),
                record.args()
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e)) // 转换为 io::Result
        })
        .init();

    let app_state = actix_web::web::Data::new(AppState::new());

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(task_routes())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await;
    info!("测试");
    Ok(())
}
