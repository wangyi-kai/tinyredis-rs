use clap::Parser;

#[derive(Parser)]
#[command(name = "tinyredis-benchmark")]
#[command(about = "A Redis benchmark tool written in Rust", long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub host: Option<String>,
    #[arg(short, long)]
    pub port: Option<u16>,
    #[arg(short, long)]
    pub clients: u32,
    #[arg(short, long)]
    pub requests: u32,
}


