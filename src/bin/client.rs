use std::cmp::Ord;
use std::str::FromStr;
use clap::{Args, Parser, Subcommand};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

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
    Init(InitArgs),
    #[command(arg_required_else_help = true)]
    Item(ItemArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    id: u8,
}

#[derive(Debug, Args)]
struct ItemArgs {
    #[arg(short = 'b', help = "Bill id to operate", value_parser = clap::value_parser!(i64).range(1..))]
    bid: i64,
    #[command(subcommand)]
    command: ItemCmds,
}

#[derive(Debug, Subcommand)]
enum ItemCmds {
    #[command(arg_required_else_help = true)]
    Add {
        #[arg(long, help = "Array of menu items to add.", value_name = "MENU_ITEM_IDs", num_args = 1..)]
        items: Vec<i64>,
    },
    #[command(arg_required_else_help = true)]
    Remove {
        #[arg(long, help = "Id of menu item to remove.", value_name = "MENU_ITEM_ID")]
        id: i64,
    },
    #[command(arg_required_else_help = true)]
    List,
}

const HOST: &str = "http://localhost:8080";

#[derive(Debug, Deserialize)]
pub(crate) struct TableInitResponse {
    pub bill_id: i64,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    match args.command {
        Commands::Table(table) => {
            match table.command {
                TableCmds::Init(args) => {
                    let id = args.id;
                    println!("initializing table={} for customers", id);
                    let res = Client::new()
                        .patch(format!("{}/{}", HOST, "v1/table/") + &id.to_string())
                        .send()
                        .await?;
                    match res.status() {
                        StatusCode::OK => {
                            let res = res.json::<TableInitResponse>().await.expect("failed to get response, aborting");
                            println!("table {} initialized successfully, bound to bill id = {}", id, res.bill_id);
                        },
                        StatusCode::BAD_REQUEST => {
                            println!("table {} is already taken, please guide the customers to other tables", id);
                        }
                        unexpected => {
                            println!("got unexpected status code, {}", unexpected);
                        },
                    }
                },
                TableCmds::Item(args) => {
                    let bill_id = args.bid;
                    match args.command {
                        ItemCmds::Add { items: menu_ids } => {
                            println!("adding items to bill={}", bill_id);
                            let res = Client::new()
                                .post(format!("{}/{}", HOST, format!("v1/bill/{}/items", bill_id)))
                                .json(&serde_json::json!({
                                    "items": menu_ids,
                                }))
                                .send()
                                .await?;
                            match res.status() {
                                StatusCode::OK => {
                                    println!("Successfully added items to bill id = {}", bill_id);
                                },
                                StatusCode::BAD_REQUEST => {
                                    println!("Failed to add items to bill id = {}", bill_id);
                                }
                                unexpected => {
                                    println!("got unexpected status code, {}", unexpected);
                                },
                            }
                        },
                        ItemCmds::Remove { id } => {
                            println!("removing items from bill={}", bill_id);
                            let res = Client::new()
                                .delete(format!("{}/{}", HOST, format!("v1/bill/{}/item/{}", bill_id, id)))
                                .send()
                                .await?;
                            match res.status() {
                                StatusCode::OK => {
                                    println!("Successfully removed items from bill id = {}", bill_id);
                                },
                                StatusCode::BAD_REQUEST => {
                                    println!("Bad request");
                                },
                                StatusCode::NOT_FOUND => {
                                    println!("Resource not found");
                                },
                                unexpected => {
                                    println!("got unexpected status code, {}", unexpected);
                                },
                            }
                        },
                        ItemCmds::List => {

                        },
                    }
                }
            }
        }
    };
    Ok(())
}