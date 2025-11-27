use crate::ext::ExtensionTrait;
use deno_core::extension;
use deno_core::op2;
use deno_core::Extension;
use deno_core::OpState;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct KvOptions {
  pub db: Option<sled::Db>,
}

extension!(
  worky_kv,
  ops = [op_kv_get, op_kv_put, op_kv_delete],
  esm_entry_point = "ext:worky_kv/01_kv.js",
  esm = [ dir "src/ext/kv", "01_kv.js" ],
  options = {
    opts: KvOptions
  },
  state = |state, config| {
    if let Some(db) = config.opts.db {
      state.put(db);
    }
  }
);

impl ExtensionTrait<KvOptions> for worky_kv {
  fn init(opts: KvOptions) -> Extension {
    worky_kv::init(opts)
  }
}

#[op2(async)]
#[string]
pub async fn op_kv_get(
  #[string] key: String,
  state: Rc<RefCell<OpState>>,
) -> Result<Option<String>, deno_core::error::CoreError> {
  let state = state.borrow();
  // TODO: Implement "?" error handling
  let db = state.try_borrow::<sled::Db>().unwrap();
  let value = db.get(key).unwrap();
  Ok(value.map(|v| String::from_utf8(v.to_vec()).unwrap_or_default()))
}

#[op2(async)]
#[string]
pub async fn op_kv_put(
  #[string] key: String,
  #[string] value: String,
  state: Rc<RefCell<OpState>>,
) -> Result<(), deno_core::error::CoreError> {
  let state = state.borrow();
  // TODO: Implement "?" error handling
  let db = state.try_borrow::<sled::Db>().unwrap();
  db.insert(key, value.as_bytes()).unwrap();
  Ok(())
}

#[op2(async)]
async fn op_kv_delete(
  #[string] key: String,
  state: Rc<RefCell<OpState>>,
) -> Result<(), deno_core::error::CoreError> {
  let state = state.borrow();
  // TODO: Implement "?" error handling
  let db = state.try_borrow::<sled::Db>().unwrap();
  db.remove(key).unwrap();
  Ok(())
}
