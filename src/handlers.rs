use actix_web::{web, HttpResponse, Responder};
use crate::models::Task;
use crate::state::AppStateData;

pub async fn get_tasks(state: AppStateData) -> impl Responder {
    let tasks = state.tasks.read().unwrap();
    let tasks: Vec<Task> = tasks.values().cloned().collect();
    HttpResponse::Ok().json(tasks)
}

pub async fn create_task(state: AppStateData, task: web::Json<Task>) -> impl Responder {
    let task = Task {
        id: task.id.clone(),
        title: task.title.clone(),
        completed: task.completed,
    };
    let mut tasks = state.tasks.write().unwrap();
    tasks.insert(task.id.clone(), task.clone());
    HttpResponse::Created().json(task)
}

pub async fn get_task(state: AppStateData, path: web::Path<String>) -> impl Responder {
    let tasks = state.tasks.read().unwrap();
    match tasks.get(&path.into_inner()) {
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().body("Task not found"),
    }
}

pub async fn delete_task(state: AppStateData, path: web::Path<String>) -> impl Responder {
    let mut tasks = state.tasks.write().unwrap();
    if tasks.remove(&path.into_inner()).is_some() {
        HttpResponse::Ok().body("Task deleted")
    } else {
        HttpResponse::NotFound().body("Task not found")
    }
}