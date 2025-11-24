use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions};
use std::io::{self, prelude::*};

mod protocol;
use protocol::*;

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

  for client in listener.incoming() {
    if let Ok(mut stream) = client {
      std::thread::spawn(move || {
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).ok();

        let request: Request = match serde_json::from_str(&buffer) {
          Ok(r) => r,
          Err(err) => {
            let _ = send_error(&mut stream, &format!("Invalid request: {}", err));
            return;
          }
        };

        let response = handle_request(request);
        let json = serde_json::to_vec(&response).unwrap();
        let _ = stream.write_all(&json);
      });
    }
  }

  Ok(())
}

fn handle_request(req: Request) -> Response {
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

      let _ = worky_store::register_worker(address, path, name);

      Response {
        status: "ok".into(),
        message: Some("Load complete".into()),
        error: None,
      }
    }
  }
}

fn send_error(stream: &mut impl Write, msg: &str) {
  let resp = Response {
    status: "error".into(),
    message: None,
    error: Some(msg.into()),
  };
  let _ = stream.write_all(serde_json::to_string(&resp).unwrap().as_bytes());
}
