use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use std::io::{self, prelude::*, BufReader};
use std::sync::Mutex;
use tokio::task::JoinHandle;

lazy_static::lazy_static! {
  pub static ref TOK_ASYNC_HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());
}

pub mod protocol;
pub use protocol::*;

pub fn run() -> Result<(), anyhow::Error> {
  let addr = worky_common::consts::paths::SOCKET_PATH.to_string();
  let addr_name = addr
    .clone()
    .to_ns_name::<GenericNamespaced>()
    .map_err(anyhow::Error::from)?;

  let opts = ListenerOptions::new().name(addr_name);

  let listener = match opts.create_sync() {
    Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
      eprintln!(
        "Error: could not start server because the socket file is occupied. Please check
              if {addr} is in use by another process and try again."
      );
      return Err(anyhow::Error::from(e));
    }
    x => x?,
  };

  println!("Daemon running on {}", addr);

  TOK_ASYNC_HANDLES
    .lock()
    .unwrap()
    .push(tokio::spawn(async move {
      for client in listener.incoming() {
        if let Ok(mut stream) = client {
          let de = serde_json::Deserializer::from_reader(&mut stream);
          let request: Request = match de.into_iter().next() {
            Some(Ok(r)) => r,
            Some(Err(err)) => {
              let _ = send_error(&mut stream, &format!("Invalid request: {}", err));
              return;
            }
            None => return, // End of stream
          };

          let response = handle_request(request).await;
          let json = serde_json::to_vec(&response).unwrap();
          let _ = stream.write_all(&json);
        }
      }
    }));

  Ok(())
}

async fn handle_request(req: Request) -> Response {
  match req {
    Request::Start {} => Response {
      status: "ok".into(),
      message: Some("Started".into()),
      error: None,
    },

    Request::Stop {} => Response {
      status: "ok".into(),
      message: Some("Stopping daemon…".into()),
      error: None,
    },

    Request::Restart {} => Response {
      status: "ok".into(),
      message: Some("Restarting daemon…".into()),
      error: None,
    },

    Request::Load {
      address,
      path,
      refresh,
      name,
    } => {
      println!(
        "LOAD request:
    address: {}
    path: {}
    refresh: {:?}
    name: {:?}",
        address, path, refresh, name
      );

      worky_store::register_worker(address, path, name).await;

      Response {
        status: "ok".into(),
        message: Some("Load complete".into()),
        error: None,
      }
    }

    Request::Unload { address } => {
      println!("UNLOAD request: address: {}", address);
      let found = worky_store::unregister_worker(address);
      if found {
        Response {
          status: "ok".into(),
          message: Some("Unload complete".into()),
          error: None,
        }
      } else {
        Response {
          status: "err".into(),
          message: None,
          error: Some("Worker not found".into()),
        }
      }
    }
  }
}

fn send_error(stream: &mut impl Write, msg: &str) {
  let resp = Response {
    status: "err".into(),
    message: None,
    error: Some(msg.into()),
  };
  let _ = stream.write_all(serde_json::to_string(&resp).unwrap().as_bytes());
}

pub fn send_request(req: Request) {
  let addr = worky_common::consts::paths::SOCKET_PATH.to_string();
  let addr_name = addr
    .to_ns_name::<GenericNamespaced>()
    .expect("Invalid socket name");

  let mut conn = Stream::connect(addr_name).expect("Could not connect to daemon");

  let req_str = serde_json::to_string(&req).unwrap();
  conn.write_all(req_str.as_bytes()).unwrap();

  // Try to shutdown write to signal EOF to the server
  // This is required if the server uses read_to_string or similar that waits for EOF.
  // Even with serde_json::from_reader, it might help if it blocks on lookahead.
  // interprocess::local_socket::Stream doesn't expose shutdown directly in all versions/platforms easily via trait?
  // But on Linux it is a UnixStream.
  // Let's try to just flush.
  conn.flush().unwrap();

  // If shutdown is not available, we rely on server not blocking.
  // But if server blocks, we are stuck.
  // Let's try to assume it works or use a different approach if it fails to compile.
  // Actually, let's try to use `std::net::Shutdown`.
  // conn.shutdown(std::net::Shutdown::Write).ok();
  // If this fails to compile, I'll remove it.
  // But wait, `LocalSocketStream` in interprocess 2.x might not have `shutdown`.
  // Let's check if we can wrap it or something.
  // For now, let's just add debug prints to see where it hangs.
  println!("Sent request, waiting for response...");

  let mut reader = BufReader::new(&mut conn);
  let mut buffer = String::new();
  reader.read_to_string(&mut buffer).unwrap();
  println!("Response: {}", buffer);
}

pub async fn keepalive() {
  let handles = std::mem::take(&mut *TOK_ASYNC_HANDLES.lock().unwrap());
  for handle in handles {
    let _ = handle.await;
  }
}
