use clap::Parser;
use worky_api::{listen_to_addr, spawn_worker};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  #[command(subcommand)]
  cmd: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
  Dev,
  Build,
}

#[tokio::main]
async fn main() {
  println!("Worky listening");
  let path = std::env::current_dir()
    .unwrap()
    .join("worky-api/test/hello.js");
  let handle = spawn_worker(
    "localhost:3000".to_string(),
    path,
    Some("worker".to_string()),
  );
  listen_to_addr("localhost:3000".to_string(), handle).await;

  let args = Args::parse();

  match args.cmd {
    Some(Commands::Dev) => println!("Dev command"),
    Some(Commands::Build) => println!("Build command"),
    None => println!("No command"),
  }
}
