use super::ResultBytes;
use hyper::{Request, Response};
use std::sync::mpsc::Sender;

pub struct WorkerRequest {
  pub resp: tokio::sync::oneshot::Sender<anyhow::Result<Response<ResultBytes>>>,
  pub request_data: Option<Request<ResultBytes>>,
}

pub struct WorkerHandle {
  pub addr: String,
  pub name: String,
  pub sender: Sender<WorkerRequest>,
}
