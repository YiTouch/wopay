use std::sync::RwLock;
use std::collections::HashMap;
use actix_web::web;
use crate::models::Task;

pub struct AppState {
    pub tasks: RwLock<HashMap<String, Task>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

pub type AppStateData = web::Data<AppState>;