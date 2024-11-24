use std::fmt::format;
use clap::{Args, Parser, Subcommand};
use log::info;
use reqwest::Client;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "bookish-eureka")]
#[command(about = "client cli used by restaurant staffs to interact with the server", version, long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Parser, Debug)]
enum Commands {
    /// table related ops
    #[command(arg_required_else_help = true)]
    Table(TableArgs),
}

#[derive(Debug, Args)]
pub(crate) struct TableArgs {
    #[command(subcommand)]
    command: TableCmds,
}

#[derive(Debug, Subcommand)]
enum TableCmds {
    #[command(arg_required_else_help = true)]
    Init(InitArgs)
}

#[derive(Debug, Args)]
pub(crate) struct InitArgs {
    id: u8,
}

const HOST: &str = "http://localhost:8080"; 

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    match args.command {
        Commands::Table(table) => {
            match table.command {
                TableCmds::Init(args) => {
                    let id = args.id;
                    info!("initializing table={} for customers", id);
                    let echo_json: serde_json::Value = Client::new()
                        .patch(format!("{}/{}", HOST, "v1/table/") + &id.to_string())
                        .json(&serde_json::json!({
                            "customer_count": 4,
                        }))
                        .send()
                        .await?
                        .json()
                        .await?;
                    info!("response={}", echo_json);
                }
            }
        }
    };
    Ok(())
}