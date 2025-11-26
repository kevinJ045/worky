use clap::Parser;
use worky_socket::{keepalive, protocol::Request as SocRequest, send_request};

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
    path: String,
    #[arg(short, long)]
    name: Option<String>,
  },
  Unload {
    #[arg(short, long)]
    address: String,
  },
  Dev,
  Build,
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  match args.cmd {
    Some(Commands::Daemon) => {
      if let Err(e) = worky_socket::run() {
        eprintln!("Daemon error: {}", e);
      }
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
    Some(Commands::Dev) => println!("Dev command"),
    Some(Commands::Build) => println!("Build command"),
    None => println!("No command"),
  }
}
