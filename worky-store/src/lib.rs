use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
  mpsc::{channel, Sender},
  Arc, Mutex,
};
use worky_runtime::{WorkerModule, WorkerRequest, WorkyRuntime};

pub struct WorkerHandle {
  addr: String,
  name: String,
  sender: Sender<WorkerRequest>,
}

pub static WORKERS: Lazy<Mutex<HashMap<String, Arc<WorkerHandle>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

pub fn spawn_worker(addr: String, module_path: String, name: Option<String>) -> WorkerHandle {
  let (tx, rx) = channel::<WorkerRequest>();
  std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    let mut runtime = WorkyRuntime::new();
    let module_future = runtime.run_module(Path::new(&module_path));
    let module_exports = rt.block_on(module_future).unwrap();

    for req in rx {
      // async runtime block
      let fut = async { Ok::<String, anyhow::Error>("done".into()) };

      let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut);

      let _ = req.resp.send(result);
    }
  });

  WorkerHandle {
    sender: tx,
    name: "".to_string(),
    addr,
  }
}
