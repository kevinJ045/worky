use clap::Parser;

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
    let args = Args::parse();

    match args.cmd {
        Some(Commands::Dev) => println!("Dev command"),
        Some(Commands::Build) => println!("Build command"),
        None => println!("No command"),
    }
}
