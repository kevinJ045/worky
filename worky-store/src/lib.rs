use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use worky_api::{listen_to_addr, spawn_worker};
use worky_common::workers::WorkerHandle;

pub static WORKERS: Lazy<Mutex<HashMap<String, Arc<WorkerHandle>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn register_worker(addr: String, path: String, name: Option<String>) {
  let handle = spawn_worker(addr.clone(), path, name);
  listen_to_addr(addr, handle).await;
}
