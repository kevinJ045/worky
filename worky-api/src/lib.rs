use axum::Router;
use deno_core::v8;
use hyper::Request;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use futures::SinkExt;

use deno_core::JsRuntime;
use worky_common::workers::{WorkerHandle, WorkerRequest};
use worky_runtime::WorkyRuntime;

pub async fn parse_js_response<'a>(
  runtime: &'a mut JsRuntime,
  res_val: v8::Global<v8::Value>,
) -> anyhow::Result<(
  hyper::Response<axum::body::Body>,
  Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>>,
)> {
  let (status, headers, body_val_global) = {
    let scope = &mut runtime.handle_scope();
    let res_val = v8::Local::new(scope, res_val);
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
    let mut headers = Vec::new();

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

      headers.push((
        key.to_rust_string_lossy(scope),
        val.to_rust_string_lossy(scope),
      ));
    }

    let body_key = v8::String::new(scope, "body").unwrap();
    let body_val = response_obj.get(scope, body_key.into()).unwrap();
    let body_val_global = v8::Global::new(scope, body_val);

    (status, headers, body_val_global)
  };

  let mut builder = hyper::Response::builder();
  for (k, v) in headers {
    builder = builder.header(k, v);
  }
  builder = builder.status(hyper::StatusCode::from_u16(status as u16).unwrap());

  let body_bytes = {
    enum BodyState {
      Done(Vec<u8>),
      Stream(v8::Global<v8::Object>),
      Error(anyhow::Error),
    }

    let body_state = {
      let scope = &mut runtime.handle_scope();
      let body_val = v8::Local::new(scope, body_val_global.clone());

      if body_val.is_string() {
        BodyState::Done(body_val.to_rust_string_lossy(scope).as_bytes().to_vec())
      } else if body_val.is_uint8_array() {
        let uint8: v8::Local<v8::Uint8Array> = body_val.try_into().unwrap();
        let len = uint8.byte_length();
        let mut buf = vec![0u8; len];
        uint8.copy_contents(&mut buf);
        BodyState::Done(buf)
      } else {
        let get_reader_key = v8::String::new(scope, "getReader").unwrap();
        let has_reader = if let Some(body_obj) = body_val.to_object(scope) {
          body_obj.has(scope, get_reader_key.into()).unwrap_or(false)
        } else {
          false
        };

        if has_reader {
          let body_obj = body_val.to_object(scope).unwrap();
          let get_reader_fn: v8::Local<v8::Function> = body_obj
            .get(scope, get_reader_key.into())
            .unwrap()
            .try_into()
            .unwrap();
          let reader_val = get_reader_fn.call(scope, body_obj.into(), &[]).unwrap();
          let reader = reader_val.to_object(scope).unwrap();
          let reader_global = v8::Global::new(scope, reader);
          BodyState::Stream(reader_global)
        } else {
          BodyState::Error(anyhow::anyhow!(
            "Response body is not String, Uint8Array, or ReadableStream"
          ))
        }
      }
    };

    match body_state {
      BodyState::Done(bytes) => (
        builder.body(axum::body::Body::from(bytes)).unwrap(),
        None,
      ),
      BodyState::Error(e) => return Err(e),
      BodyState::Stream(reader_global) => {
        let (mut tx, rx) = futures::channel::mpsc::channel::<Result<axum::body::Bytes, anyhow::Error>>(10);
        let body = axum::body::Body::from_stream(rx);

        let pumper: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> = Box::pin(async move {
            let read_key_str = "read";

            loop {
              let promise_global = {
                let scope = &mut runtime.handle_scope();
                let read_key = v8::String::new(scope, read_key_str).unwrap();
                let reader = v8::Local::new(scope, reader_global.clone());
                let read_fn: v8::Local<v8::Function> = reader
                  .get(scope, read_key.into())
                  .unwrap()
                  .try_into()
                  .unwrap();
                let promise_val = read_fn.call(scope, reader.into(), &[]).unwrap();
                let promise: v8::Local<v8::Promise> = promise_val.try_into().unwrap();
                let promise_value: v8::Local<v8::Value> = promise.into();
                v8::Global::new(scope, promise_value)
              };

              #[allow(deprecated)]
              let result_global_res = runtime.resolve_value(promise_global).await;
              
              let result_global = match result_global_res {
                  Ok(r) => r,
                  Err(e) => {
                      let _ = tx.send(Err(e.into())).await;
                      break;
                  }
              };

              let scope = &mut runtime.handle_scope();
              let result: v8::Local<v8::Value> = v8::Local::new(scope, result_global);
              let result_obj = result.to_object(scope).unwrap();

              let done_key = v8::String::new(scope, "done").unwrap();
              let done = result_obj.get(scope, done_key.into()).unwrap();
              if done.is_true() {
                break;
              }

              let value_key = v8::String::new(scope, "value").unwrap();
              let value = result_obj.get(scope, value_key.into()).unwrap();
              let uint8: v8::Local<v8::Uint8Array> = value.try_into().unwrap();
              let len = uint8.byte_length();
              let mut buf = vec![0u8; len];
              uint8.copy_contents(&mut buf);
              
              if tx.send(Ok(axum::body::Bytes::from(buf))).await.is_err() {
                  break;
              }
            }
        });
        
        (builder.body(body).unwrap(), Some(pumper))
      }
    }
  };

  Ok(body_bytes)
}

pub fn spawn_worker(
  addr: String,
  module_path: impl Into<PathBuf>,
  name: Option<String>,
) -> WorkerHandle {
  let (tx, rx) = channel::<WorkerRequest>();
  let path = module_path.into();
  let addr_r = addr.clone();
  let name_r = name.clone();
  std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    let mut runtime = WorkyRuntime::new(Some(addr_r.clone()), name_r);
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
        let res_global_result = {
          let scope = &mut runtime.js_runtime.handle_scope();
          println!("Global: {fetch_global:?}");
          if let Some(fetch_global) = &fetch_global {
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

            match js_result {
              Ok(r) => Ok(v8::Global::new(scope, r)),
              Err(e) => Err(e),
            }
          } else {
            Err(anyhow::anyhow!("fetch() is not defined"))
          }
        };

        match res_global_result {
          Ok(res_global) => {
            // Check if it is a promise and await it
            let res_promise_opt = {
              let scope = &mut runtime.js_runtime.handle_scope();
              let local = v8::Local::new(scope, res_global.clone());
              if local.is_promise() {
                let promise: v8::Local<v8::Promise> = local.try_into().unwrap();
                let promise_value: v8::Local<v8::Value> = promise.into();
                Some(v8::Global::new(scope, promise_value))
              } else {
                None
              }
            };

            let final_res = if let Some(p) = res_promise_opt {
              runtime.js_runtime.resolve(p).await.unwrap()
            } else {
              res_global
            };

            let (res, pumper) = parse_js_response(&mut runtime.js_runtime, final_res).await?;
            Ok((res, pumper))
          }
          Err(e) => Err(e),
        }
      };

      let result = rt.block_on(fut);
      
      match result {
          Ok((res, pumper)) => {
              let _ = req.resp.send(Ok(res));
              if let Some(pumper) = pumper {
                  rt.block_on(pumper);
              }
          }
          Err(e) => {
              let _ = req.resp.send(Err(e));
          }
      }
    }
  });

  WorkerHandle {
    sender: tx,
    name: name.unwrap_or("".to_string()),
    addr,
  }
}

pub async fn listen_to_addr(addr: String, handle: std::sync::Arc<WorkerHandle>) {
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
      resp
    }
  });

  axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
  use super::*;
  use deno_core::v8;
  use worky_runtime::WorkyRuntime;

  #[tokio::test]
  async fn test_parse_readable_stream() {
    let mut runtime = WorkyRuntime::new(None, None);
    let code = r#"
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new Uint8Array([72, 101, 108, 108, 111])); // Hello
          controller.enqueue(new Uint8Array([32, 87, 111, 114, 108, 100])); // World
          controller.close();
        }
      });
      new Response(stream)
    "#;

    let res_global = {
      let scope = &mut runtime.js_runtime.handle_scope();
      let code = v8::String::new(scope, code).unwrap();
      let script = v8::Script::compile(scope, code, None).unwrap();
      let result = script.run(scope).unwrap();
      v8::Global::new(scope, result)
    };

    let (response, pumper) = parse_js_response(&mut runtime.js_runtime, res_global)
      .await
      .unwrap();
    
    if let Some(pumper) = pumper {
        // We need to drive the pumper while reading the body
        let body = response.into_body();
        use http_body_util::BodyExt;
        
        let (body_res, _) = tokio::join!(
            body.collect(),
            pumper
        );
        let body_bytes = body_res.unwrap().to_bytes();
        assert_eq!(body_bytes.as_ref(), b"Hello World");
    } else {
        panic!("Expected stream");
    }
  }
}
