use axum::{body::Bytes, Router};
use deno_core::v8;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use worky_common::workers::{WorkerHandle, WorkerRequest};
use worky_runtime::WorkyRuntime;

pub fn parse_js_response_sync<'s>(
  scope: &mut v8::HandleScope<'s>,
  res_val: v8::Local<v8::Value>,
) -> anyhow::Result<hyper::Response<hyper::body::Bytes>> {
  let response_obj: v8::Local<v8::Object> = res_val
    .try_into()
    .map_err(|_| anyhow::anyhow!("fetch() returned non-object"))?;

  let status_key = v8::String::new(scope, "status").unwrap();
  let status_val = response_obj.get(scope, status_key.into()).unwrap();
  let status = status_val.uint32_value(scope).unwrap_or(200);

  let headers_key = v8::String::new(scope, "headers").unwrap();
  let js_headers_val = response_obj.get(scope, headers_key.into()).unwrap();
  let js_headers: v8::Local<v8::Object> = js_headers_val
    .try_into()
    .expect("Response.headers must be an object");

  let entries_key = v8::String::new(scope, "entries").unwrap();
  let entries_fn: v8::Local<v8::Function> = js_headers
    .get(scope, entries_key.into())
    .unwrap()
    .try_into()
    .unwrap();

  let entries_iter_val = entries_fn.call(scope, js_headers.into(), &[]).unwrap();
  let entries_iter: v8::Local<v8::Object> = entries_iter_val
    .try_into()
    .expect("Response.headers must be an object");

  let next_key = v8::String::new(scope, "next").unwrap();

  let mut builder = hyper::Response::builder();

  loop {
    let next_fn: v8::Local<v8::Function> = entries_iter
      .get(scope, next_key.into())
      .unwrap()
      .try_into()
      .unwrap();

    let item = next_fn.call(scope, entries_iter.into(), &[]).unwrap();
    let item_obj = v8::Local::<v8::Object>::try_from(item).unwrap();

    let done_key = v8::String::new(scope, "done").unwrap();
    let done_val = item_obj.get(scope, done_key.into()).unwrap();
    if done_val.is_true() {
      break;
    }

    let value_key = v8::String::new(scope, "value").unwrap();
    let pair = item_obj.get(scope, value_key.into()).unwrap();
    let pair_arr = v8::Local::<v8::Array>::try_from(pair).unwrap();

    let key = pair_arr.get_index(scope, 0).unwrap();
    let val = pair_arr.get_index(scope, 1).unwrap();

    builder = builder.header(
      key.to_rust_string_lossy(scope),
      val.to_rust_string_lossy(scope),
    );
  }

  let body_key = v8::String::new(scope, "body").unwrap();
  let body_val = response_obj.get(scope, body_key.into()).unwrap();

  let uint8: v8::Local<v8::Uint8Array> = body_val
    .try_into()
    .map_err(|_| anyhow::anyhow!("Response body must be Uint8Array"))?;

  let len = uint8.byte_length();
  let mut buf = vec![0u8; len];
  uint8.copy_contents(&mut buf);

  // Build hyper response
  builder = builder.status(hyper::StatusCode::from_u16(status as u16).unwrap());
  Ok(builder.body(buf.into()).unwrap())
}

// pub async fn parse_js_response_arraybuffer<'s>(
//   scope: &mut v8::HandleScope<'s>,
//   res_val: v8::Local<'s, v8::Value>,
//   runtime: &mut deno_core::JsRuntime,
// ) -> anyhow::Result<hyper::Response<Vec<u8>>> {
//   let response_obj: v8::Local<v8::Object> = res_val.try_into()?;

//   // ----- call response.arrayBuffer() -----
//   let arraybuffer_key = v8::String::new(scope, "arrayBuffer").unwrap();
//   let ab_fn: v8::Local<v8::Function> = response_obj
//     .get(scope, arraybuffer_key.into())
//     .unwrap()
//     .try_into()
//     .unwrap();

//   let promise_val = ab_fn.call(scope, response_obj.into(), &[]).unwrap();

//   let promise: v8::Local<v8::Promise> = promise_val.try_into()?;

//   // ---- await JS promise ----
//   runtime.await_promise(scope, promise).await?;

//   let result_val = promise.result(scope);
//   let ab: v8::Local<v8::ArrayBuffer> = result_val.try_into()?;

//   let store = ab.get_backing_store();
//   let data = store.data();
//   let slice = unsafe { std::slice::from_raw_parts(data as *const u8, store.byte_length()) };

//   // now parse status + headers using the same code as case 1
//   let mut resp = parse_js_response_sync(scope, res_val)?;
//   *resp.body_mut() = slice.to_vec();
//   Ok(resp)
// }

pub fn spawn_worker(
  addr: String,
  module_path: impl Into<PathBuf>,
  name: Option<String>,
) -> WorkerHandle {
  let (tx, rx) = channel::<WorkerRequest>();
  let path = module_path.into();
  let addr_r = addr.clone();
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
      let fut = async {
        let scope = &mut runtime.js_runtime.handle_scope();
        match if let Some(fetch_global) = &fetch_global {
          let js_result = {
            let js_request_obj = {
              let req_data = req.request_data.unwrap();

              let req_body = req_data.body();
              let mut body_bytes = req_body.to_vec();
              let body_len = body_bytes.len();

              let slice: &mut [u8] = &mut body_bytes[..];

              let uint8 = {
                let ab = v8::ArrayBuffer::new(scope, body_len);
                let uint8 = v8::Uint8Array::new(scope, ab, 0, body_len).unwrap();

                uint8.copy_contents(slice);
                uint8
              };

              let global = scope.get_current_context().global(scope);

              let req_key = v8::String::new(scope, "Request").unwrap();
              let request_ctor: v8::Local<v8::Function> = global
                .get(scope, req_key.into())
                .unwrap()
                .try_into()
                .unwrap();

              let url = v8::String::new(
                scope,
                &format!("http://{}{}", addr_r, req_data.uri().to_string()),
              )
              .unwrap();
              let init_obj = v8::Object::new(scope);

              let meth = req_data.method();

              let method = v8::String::new(scope, meth.as_str()).unwrap();
              let met_key = v8::String::new(scope, "method").unwrap();
              init_obj.set(scope, met_key.into(), method.into());

              let headers_key = v8::String::new(scope, "Headers").unwrap();
              let headers_ctor: v8::Local<v8::Function> = global
                .get(scope, headers_key.into())
                .unwrap()
                .try_into()
                .unwrap();

              let js_headers = headers_ctor.new_instance(scope, &[]).unwrap();

              let append_key = v8::String::new(scope, "append").unwrap();
              let append_fn: v8::Local<v8::Function> = js_headers
                .get(scope, append_key.into())
                .unwrap()
                .try_into()
                .unwrap();

              for (key, value) in req_data.headers().iter() {
                let k = v8::String::new(scope, key.as_str()).unwrap();
                let v = v8::String::new(scope, value.to_str().unwrap()).unwrap();

                append_fn.call(scope, js_headers.into(), &[k.into(), v.into()]);
              }

              let headers_init_key = v8::String::new(scope, "headers").unwrap();
              init_obj.set(scope, headers_init_key.into(), js_headers.into());

              if meth != "GET" && meth != "HEAD" {
                let bod_key = v8::String::new(scope, "body").unwrap();
                init_obj.set(scope, bod_key.into(), uint8.into());
              }

              request_ctor
                .new_instance(scope, &[url.into(), init_obj.into()])
                .unwrap()
            };

            let func = fetch_global.open(scope);

            let recv = v8::undefined(scope).into();
            let call_result = func.call(scope, recv, &[js_request_obj.into()]);

            match call_result {
              Some(res) => Ok(res),
              None => Err(anyhow::anyhow!("fetch() threw an exception")),
            }
          };

          Ok::<v8::Local<v8::Value>, anyhow::Error>(js_result.expect("JS execution failed"))
        } else {
          Err(anyhow::anyhow!("fetch() is not defined"))
        } {
          Ok(r) => parse_js_response_sync(scope, r),
          Err(e) => Err(e),
        }
      };

      let result = rt.block_on(fut);

      let _ = req.resp.send(result);
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
        body_bytes.to_vec().into()
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
