use hyper::{Request, Response};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{mpsc::Sender, Arc, Mutex};
use worky_common::ResultBytes;

pub struct WorkerRequest {
  pub resp: tokio::sync::oneshot::Sender<anyhow::Result<Response<ResultBytes>>>,
  pub request_data: Option<Request<ResultBytes>>,
}

pub struct WorkerHandle {
  pub addr: String,
  pub name: String,
  pub sender: Sender<WorkerRequest>,
}

pub static WORKERS: Lazy<Mutex<HashMap<String, Arc<WorkerHandle>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));
