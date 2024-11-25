use clap::{Args, Parser, Subcommand};
use reqwest::{Client, StatusCode};
use serde::{Deserialize};

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
    #[command(subcommand)]
    command: ItemCmds,
    #[arg(short = 't', help = "Table id to operate")]
    tid: u8,
}

#[derive(Debug, Subcommand)]
enum ItemCmds {
    #[command(arg_required_else_help = true)]
    Add {
        #[arg(long, help = "Array of items to add.", value_name = "ITEMS", value_parser(parse_item_vec))]
        items: Vec<Item>,
    },
    #[command(arg_required_else_help = true)]
    Remove{
        #[arg(long, help = "Array of items to remove.", value_name = "ITEMS", value_parser(parse_item_vec))]
        items: Vec<Item>,
    },
    #[command(arg_required_else_help = true)]
    List,
}

#[derive(Debug, Args)]
struct ItemAddArgs {
    #[arg(long, help = "Array of items to add.", value_name = "ITEMS", value_parser(parse_item_vec))]
    items: Vec<Item>,
}

#[derive(Debug, Args)]
struct ItemRemoveArgs {
    #[arg(long, help = "Array of items to remove.", value_name = "ITEMS", value_parser(parse_item_vec))]
    items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
struct Item {
    id: u16,
    count: u16,
}

fn parse_item_vec(val: &str) -> Result<Vec<Item>, String> {
    let rules = serde_json::from_str(val).map_err(|e| e.to_string())?;
    Ok(rules)
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
                        .json(&serde_json::json!({
                            "customer_count": 4,
                        }))
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

                }
            }
        }
    };
    Ok(())
}