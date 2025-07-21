use actix_web::{web, Scope};
use crate::handlers::*;

pub fn task_routes() -> Scope {
    web::scope("/tasks")
        .route("/task", web::get().to(get_tasks))
        .route("/task", web::post().to(create_task))
        .route("/{id}", web::get().to(get_task))
        .route("/{id}", web::delete().to(delete_task))
}