mod loader;
pub mod pool;

pub use pool::IsolatePool;

use anyhow::Result;
use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::ModuleSpecifier;
use deno_core::RuntimeOptions;
use std::path::Path;
use std::rc::Rc;

pub struct WorkyRuntime {
  pub js_runtime: JsRuntime,
}

impl WorkyRuntime {
  pub fn new() -> Self {
    let loader = Rc::new(loader::FsModuleLoader);
    let mut options = RuntimeOptions::default();
    options.module_loader = Some(loader);
    options.extensions = worky_ops::init_ops();

    let js_runtime = JsRuntime::new(options);
    Self { js_runtime }
  }

  pub async fn run(&mut self, code: &str) -> Result<()> {
    let _ = self.js_runtime.execute_script("<anon>", code.to_string())?;
    self.js_runtime.run_event_loop(Default::default()).await?;
    Ok(())
  }

  pub async fn run_module(&mut self, file_path: &Path) -> Result<v8::Global<v8::Object>> {
    let main_module = ModuleSpecifier::from_file_path(file_path)
      .map_err(|_| anyhow::anyhow!("Invalid file path"))?;
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
    let mut runtime = WorkyRuntime::new();
    let result = runtime.run("console.log('Hello from JS')").await;
    match &result {
      Err(err) => eprintln!("{err}"),
      _ => {}
    }
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_run_module() {
    let mut runtime = WorkyRuntime::new();
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
    let mut runtime = WorkyRuntime::new();
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
}
