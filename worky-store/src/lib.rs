use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use worky_api::{listen_to_addr, spawn_worker};
use worky_common::workers::WorkerHandle;

pub static WORKERS: Lazy<Mutex<HashMap<String, Arc<WorkerHandle>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));
lazy_static::lazy_static! {
  pub static ref LISTENER_HANDLES: Mutex<HashMap<String, JoinHandle<()>>> = Mutex::new(HashMap::new());
}

pub async fn register_worker(addr: String, path: PathBuf, name: Option<String>) {
  println!("Worker registered {addr} from {path:?} as  {name:?}!!");
  let handle = spawn_worker(addr.clone(), path, name);
  let handle = Arc::new(handle);
  WORKERS.lock().unwrap().insert(addr.clone(), handle.clone());

  listen_to_addr(addr, handle).await;
}

pub fn unregister_worker(addr: String) -> bool {
  // TODO: We need a way to stop the listener.
  // For now, we just remove it from the map, but the listener task is still running.
  // In a real implementation, we would need a shutdown signal.
  // However, since listen_to_addr binds to a port, we can't easily stop it without a signal.
  // Let's assume for this iteration we just remove it from tracking.
  // Actually, `listen_to_addr` runs `axum::serve`, which can be graceful shutdown.
  // But `worky-api` doesn't expose a shutdown mechanism yet.
  // I will add a TODO and just remove from map for now.
  LISTENER_HANDLES.lock().unwrap().get(&addr).unwrap().abort();
  WORKERS.lock().unwrap().remove(&addr).is_some()
}
