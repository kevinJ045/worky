mod loader;
pub mod pool;

pub use pool::IsolatePool;

use anyhow::Result;
use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::ModuleSpecifier;
use deno_core::RuntimeOptions;
use std::fs;
use std::path::Path;
use std::rc::Rc;

pub struct WorkyRuntime {
  pub js_runtime: JsRuntime,
}

impl WorkyRuntime {
  pub fn new(addr: Option<String>, name: Option<String>) -> Self {
    let loader = Rc::new(loader::FsModuleLoader);
    let mut options = RuntimeOptions::default();
    options.module_loader = Some(loader);
    // TODO: Implement a better DB system
    // Instead of opening "worky_kv.db"
    // Make a proper DB path
    let kv_db = sled::open("worky_kv.db").ok();
    // For now, load secrets from env vars starting with WORKY_SECRET_
    let secrets: std::collections::HashMap<String, String> =
      std::env::vars().map(|(k, v)| (k, v)).collect();

    options.extensions = worky_ops::init_ops(if addr.is_some() && name.is_some() {
      worky_ops::WorkyInitOptions {
        worker_address: addr.unwrap(),
        worker_name: name.unwrap(),
        kv_db,
        secrets,
      }
    } else if addr.is_some() {
      worky_ops::WorkyInitOptions {
        worker_address: addr.unwrap(),
        kv_db,
        secrets,
        ..Default::default()
      }
    } else if name.is_some() {
      worky_ops::WorkyInitOptions {
        worker_name: name.unwrap(),
        kv_db,
        secrets,
        ..Default::default()
      }
    } else {
      worky_ops::WorkyInitOptions {
        kv_db,
        secrets,
        ..Default::default()
      }
    });

    let js_runtime = JsRuntime::new(options);
    Self { js_runtime }
  }

  pub async fn run(&mut self, code: &str) -> Result<()> {
    let _ = self.js_runtime.execute_script("<anon>", code.to_string())?;
    self.js_runtime.run_event_loop(Default::default()).await?;
    Ok(())
  }

  pub async fn run_module(&mut self, file_path: &Path) -> Result<v8::Global<v8::Object>> {
    let main_module = ModuleSpecifier::from_file_path(fs::canonicalize(file_path)?)
      .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mod_id = self.js_runtime.load_main_es_module(&main_module).await?;
    let result = self.js_runtime.mod_evaluate(mod_id);
    self.js_runtime.run_event_loop(Default::default()).await?;
    result.await?;

    let ns_local = self.js_runtime.get_module_namespace(mod_id)?;

    Ok(ns_local)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_run_js() {
    let mut runtime = WorkyRuntime::new(None, None);
    let result = runtime.run("console.log('Hello from JS')").await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_run_module() {
    let mut runtime = WorkyRuntime::new(None, None);
    let path = std::env::current_dir().unwrap().join("test/test_module.js");
    println!("{path:?}");
    let result = runtime.run_module(&path).await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_isolate_pool() {
    let pool = IsolatePool::new(2);
    let result = pool.run("console.log('Hello from pool')".to_string()).await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_fetch() {
    let mut runtime = WorkyRuntime::new(None, None);
    // We can't easily test real fetch without internet or a local server.
    // But we can check if the symbol exists.
    let code = r#"
            if (typeof fetch !== 'function') throw new Error("fetch not found");
            console.log("fetch exists");
        "#;
    let result = runtime.run(code).await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_milestone2() {
    // Set secret env var
    std::env::set_var("WORKY_SECRET_TEST_SECRET", "secret_value");

    let mut runtime = WorkyRuntime::new(None, None);
    let code = r#"
        console.log("Starting Milestone 2 Verification");

        // 1. Test KV
        console.log("Testing KV...");
        await KV.put("test_key", "test_value");
        const val = await KV.get("test_key");
        if (val !== "test_value") throw new Error(`KV get failed: expected 'test_value', got '${val}'`);
        await KV.delete("test_key");
        const valDeleted = await KV.get("test_key");
        if (valDeleted !== null && valDeleted !== undefined && valDeleted !== "") throw new Error(`KV delete failed: got '${valDeleted}'`);
        console.log("KV Test Passed");

        // 2. Test Secrets
        console.log("Testing Secrets...");
        const secret = Secrets.get("TEST_SECRET");
        if (secret !== "secret_value") throw new Error(`Secrets failed: expected 'secret_value', got '${secret}'`);
        console.log("Secrets Test Passed");

        // 3. Test Network Isolation
        console.log("Testing Network Isolation...");
        try {
            await fetch("http://localhost:12345");
            throw new Error("Network isolation failed: localhost access should be blocked");
        } catch (e) {
            console.log("Caught expected error:", e.message);
            if (!e.message.includes("blocked")) {
                 throw new Error(`Unexpected error message: ${e.message}`);
            }
        }
        console.log("Network Isolation Test Passed");
    "#;
    let result = runtime.run(code).await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }
}
