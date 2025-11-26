use std::sync::Mutex;

use super::{worky::WorkerState, ExtensionTrait};
use deno_core::{extension, op2, Extension, OpState};
use once_cell::sync::Lazy;
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogType {
  Error,
  Info,
}

pub static LOGS: Lazy<
  Mutex<
    Vec<(
      String, /* addr */
      String, /* name */
      String, /* logs */
      LogType,
    )>,
  >,
> = Lazy::new(|| Mutex::new(Vec::new()));

extension!(
    init_console,
    deps = [worky_js],
    ops = [
      op_log_stdout,
      op_log_stderr
    ],
    esm_entry_point = "ext:init_console/init_console.js",
    esm = [ dir "src/ext/console", "init_console.js" ],
);
impl ExtensionTrait<()> for init_console {
  fn init((): ()) -> Extension {
    init_console::init()
  }
}
impl ExtensionTrait<()> for deno_console::deno_console {
  fn init((): ()) -> Extension {
    deno_console::deno_console::init()
  }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
  vec![
    deno_console::deno_console::build((), is_snapshot),
    init_console::build((), is_snapshot),
  ]
}
#[op2(fast)]
fn op_log_stdout(state: &mut OpState, #[string] out: String) {
  let worker = state.borrow::<WorkerState>();

  if let Ok(mut logs) = LOGS.lock() {
    logs.push((
      worker.worker_address.clone(),
      worker.worker_name.clone(),
      out.trim_end().to_owned(),
      LogType::Info,
    ));
  }
}

#[op2(fast)]
fn op_log_stderr(state: &mut OpState, #[string] out: String) {
  let worker = state.borrow::<WorkerState>();

  if let Ok(mut logs) = LOGS.lock() {
    logs.push((
      worker.worker_address.clone(),
      worker.worker_name.clone(),
      out.trim_end().to_owned(),
      LogType::Error,
    ));
  }
}

pub fn get_logs(query: String) -> Vec<(String, String, String, LogType)> {
  let query = query.trim().to_lowercase();
  let filters: Vec<&str> = query.split_whitespace().filter(|s| !s.is_empty()).collect();

  if filters.is_empty() {
    return Vec::new();
  }

  let logs = LOGS.lock().unwrap_or_else(|e| e.into_inner());

  logs
    .iter()
    .filter(|(addr, name, msg, level)| {
      let msg_low = msg.to_lowercase();
      let addr_low = addr.to_lowercase();
      let name_low = name.to_lowercase();

      for &filter in &filters {
        let matched = if let Some(addr_part) = filter.strip_prefix("addr:") {
          addr_low.contains(addr_part)
        } else if let Some(name_part) = filter.strip_prefix("name:") {
          name_low.contains(name_part)
        } else {
          false
        };

        if !matched {
          return false;
        }
      }
      true
    })
    .cloned()
    .collect()
}
