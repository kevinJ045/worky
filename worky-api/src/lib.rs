use axum::Router;
use deno_core::v8;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use worky_common::workers::{WorkerHandle, WorkerRequest};
use worky_runtime::WorkyRuntime;

pub fn spawn_worker(
  addr: String,
  module_path: impl Into<PathBuf>,
  name: Option<String>,
) -> WorkerHandle {
  let (tx, rx) = channel::<WorkerRequest>();
  let path = module_path.into();
  std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    let mut runtime = WorkyRuntime::new();
    let module_future = runtime.run_module(Path::new(&path));

    let fetch_global = {
      let module_exports = rt.block_on(module_future);
      if let Ok(module_exports) = module_exports {
        let scope = &mut runtime.js_runtime.handle_scope();
        let ns_local = module_exports.open(scope);

        let default_key = v8::String::new(scope, "default").unwrap();
        let default_val = ns_local.get(scope, default_key.into()).unwrap();

        let default_obj =
          v8::Local::<v8::Object>::try_from(default_val).expect("default export must be an object");

        let fetch_key = v8::String::new(scope, "fetch").unwrap();
        let fetch_val = default_obj.get(scope, fetch_key.into()).unwrap();

        if !fetch_val.is_function() {
          None
        } else {
          let func = v8::Local::<v8::Function>::try_from(fetch_val).unwrap();
          Some(v8::Global::new(scope, func))
        }
      } else {
        eprintln!("Error: {}", module_exports.err().unwrap());
        None
      }
    };

    for req in rx {
      let input = req.request_data.unwrap_or_default();
      let fut = async {
        if let Some(fetch_global) = &fetch_global {
          let js_result = {
            let scope = &mut runtime.js_runtime.handle_scope();

            let func = fetch_global.open(scope);

            let js_arg = v8::String::new(scope, "Hello!").unwrap();

            let recv = v8::undefined(scope).into();
            let call_result = func.call(scope, recv, &[js_arg.into()]);

            match call_result {
              Some(res) => {
                if let Some(str_val) = res.to_rust_string_lossy(scope).into() {
                  Ok(str_val)
                } else {
                  Ok("<non-string result>".into())
                }
              }
              None => Err(anyhow::anyhow!("fetch() threw an exception")),
            }
          };

          Ok::<String, anyhow::Error>(js_result.expect("JS execution failed"))
        } else {
          Err(anyhow::anyhow!("fetch() is not defined"))
        }
      };

      let result = rt.block_on(fut);

      let _ = req
        .resp
        .send(Ok(Response::new(result.unwrap().as_bytes().to_vec())));
    }
  });

  WorkerHandle {
    sender: tx,
    name: name.unwrap_or("".to_string()),
    addr,
  }
}

pub async fn listen_to_addr(addr: String, handle: WorkerHandle) {
  let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
  let handle = std::sync::Arc::new(handle);

  let app = Router::new().fallback(move |req: Request<axum::body::Body>| {
    let handle = handle.clone();
    async move {
      let (tx, rx) = tokio::sync::oneshot::channel();

      let req_bytes = req.map(|body| {
        use http_body_util::BodyExt;
        let body_bytes = futures::executor::block_on(body.collect())
          .unwrap()
          .to_bytes();
        body_bytes.to_vec()
      });

      let worker_req = WorkerRequest {
        resp: tx,
        request_data: Some(req_bytes),
      };

      handle.sender.send(worker_req).unwrap();

      let resp = rx.await.unwrap().unwrap();

      let (parts, body) = resp.into_parts();
      Response::from_parts(parts, axum::body::Body::from(body))
    }
  });

  axum::serve(listener, app).await.unwrap();
}
