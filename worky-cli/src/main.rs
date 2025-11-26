use std::path::PathBuf;

use clap::Parser;
use worky_socket::{keepalive, protocol::Request as SocRequest, send_request};

use tracing::{error, info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  #[command(subcommand)]
  cmd: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
  Daemon,
  Load {
    #[arg(short, long)]
    address: String,
    #[arg(short, long)]
    path: PathBuf,
    #[arg(short, long)]
    name: Option<String>,
  },
  Unload {
    #[arg(short, long)]
    address: String,
  },
  Log {
    #[arg()]
    query: String,
  },
  Dev,
  Build,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
  color_eyre::install().map_err(anyhow::Error::msg)?;

  tracing_subscriber::fmt()
    .with_max_level(Level::TRACE)
    .with_span_events(FmtSpan::ACTIVE)
    .with_ansi(true)
    .init();

  let args = Args::parse();

  match args.cmd {
    Some(Commands::Daemon) => {
      if let Err(e) = worky_socket::run() {
        eprintln!("Daemon error: {}", e);
      }
      send_request(SocRequest::Load {
        address: "localhost:3000".to_string(),
        path: std::env::current_dir()
          .unwrap()
          .join("worky-api/test/hello.js"),
        refresh: None,
        name: Some("worker1".to_string()),
      });
      keepalive().await;
    }
    Some(Commands::Load {
      address,
      path,
      name,
    }) => {
      send_request(SocRequest::Load {
        address,
        path,
        refresh: None,
        name,
      });
    }
    Some(Commands::Unload { address }) => {
      send_request(SocRequest::Unload { address });
    }
    Some(Commands::Log { query }) => {
      let logs = worky_ops::ext::console::get_logs(query);

      for (addr, name, logs, level) in logs {
        match level {
          worky_ops::ext::console::LogType::Error => error!(
            worker = name,
            addr = %addr,
            "{logs}"
          ),
          worky_ops::ext::console::LogType::Info => info!(
            worker = name,
            addr = %addr,
            "{logs}"
          ),
        }
      }
    }
    Some(Commands::Dev) => println!("Dev command"),
    Some(Commands::Build) => println!("Build command"),
    None => println!("No command"),
  };

  Ok(())
}
