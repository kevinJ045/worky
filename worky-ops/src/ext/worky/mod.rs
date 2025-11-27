use super::ExtensionTrait;
use deno_core::{extension, op2, Extension, OpState};

#[derive(Default, Clone)]
pub struct WorkyInitOptions {
  pub worker_name: String,
  pub worker_address: String,
  pub kv_db: Option<sled::Db>,
  pub secrets: std::collections::HashMap<String, String>,
}

pub struct WorkerState {
  pub worker_name: String,
  pub worker_address: String,
}

extension!(
  worky_js,
  ops = [op_tls_peer_certificate],
  esm = [ dir "src/ext/worky", "utils.js" ],
  options = {
    opts: WorkyInitOptions
  },
  state = |state, config| state.put(WorkerState {
    worker_name: config.opts.worker_name.clone(),
    worker_address: config.opts.worker_address.clone(),
  })
);
impl ExtensionTrait<WorkyInitOptions> for worky_js {
  fn init(opts: WorkyInitOptions) -> Extension {
    worky_js::init(opts)
  }
}

#[op2]
#[string]
fn op_tls_peer_certificate() -> String {
  "".to_string()
}
