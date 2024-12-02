use std::fmt::Formatter;
use std::ops::RangeInclusive;
use std::time::Duration;
use clap::{Args, Parser, Subcommand};
use derive_more::Display;
use rand::{thread_rng, Rng};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize};
use tokio::{pin, select};

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
    /// integration test
    #[command(arg_required_else_help = true)]
    Test(TestArgs)
}

#[derive(Debug, Args)]
pub(crate) struct TestArgs {
    concurrency: u32,
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
    List
}

#[derive(Debug, Args)]
struct InitArgs {
    id: u8,
}

#[derive(Debug, Args)]
struct ItemArgs {
    #[arg(short = 'b', help = "Bill id to operate", value_name = "BILL_ID", value_parser = clap::value_parser!(i64).range(1..))]
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
    List,
}

const HOST: &str = "http://localhost:8080";

#[derive(Debug, Deserialize)]
struct TableInitResponse {
    pub bill_id: i64,
}

#[derive(Debug, Deserialize)]
struct GetBillResponse {
    pub bill: Option<Bill>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GetTablesResponse {
    pub tables: Tables,
}

#[derive(Debug, Deserialize)]
struct Tables(Option<Vec<Table>>);

#[derive(Debug, Deserialize)]
pub(crate) struct Table {
    pub id: u8,
    pub bill_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Bill {
    pub id: i64,
    pub items: Items,
}

#[derive(Debug, Deserialize, Default)]
struct Items(Vec<Item>);

#[derive(Debug, Deserialize)]
pub(crate) struct Item {
    pub id: i64,
    pub name: String,
    pub time_to_deliver: i32,
    pub state: String,
}

impl Display for Items {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "[]");
        }
        for (i, item) in self.0.iter().enumerate() {
            write!(f, "\n")?;
            let maybe_comma = if i == self.0.len() - 1 { "" } else { ", " };  
            write!(f, "{{ id: {}, name: {}, time_to_deliver: {} mins, state: {} }}{}", item.id, item.name, item.time_to_deliver, item.state, maybe_comma)?;
        }
        Ok(())
    }
}

impl Display for Tables {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_none() || self.0.as_ref().unwrap().is_empty() {
            return write!(f, "[]");
        }
        let tables = self.0.as_ref().unwrap();
        for (i, table) in tables.iter().enumerate() {
            write!(f, "\n")?;
            let maybe_comma = if i == tables.len() - 1 { "" } else { ", " };
            write!(f, "{{ id: {}, bill_id: {} }}{}", table.id, table.bill_id.map_or_else(|| "x".to_string(), |v| v.to_string()), maybe_comma)?;
        }
        Ok(())
    }
}

const TABLE_ID_RANGE: RangeInclusive<i32> = 1..=10; // match V1__init.sql

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
                            println!("listing items from bill={}", bill_id);
                            let res = Client::new()
                                .get(format!("{}/{}", HOST, format!("v1/bill/{}", bill_id)))
                                .send()
                                .await?;
                            match res.status() {
                                StatusCode::OK => {
                                    let res = res.json::<GetBillResponse>().await?;
                                    println!("items for bill = [{}] => {}", bill_id, res.bill.map(|b|b.items).unwrap_or(Items::default()));
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
                    }
                },
                TableCmds::List => {
                    println!("Listing tables =>");
                    let res = Client::new()
                        .get(format!("{}/{}", HOST, format!("v1/tables")))
                        .send()
                        .await?;
                    match res.status() {
                        StatusCode::OK => {
                            let tables = res.json::<GetTablesResponse>().await?.tables;
                            println!("{}", tables);
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
                }
            }
        },
        Commands::Test(TestArgs{ concurrency}) => {
            let intval = tokio::time::interval(Duration::new(0, 500_000_000)); // emit a batch of request every 0.5 second
            pin!(intval);
            loop {
                select! {
                    _ = intval.tick() => {},
                    _ = tokio::signal::ctrl_c() => {
                        println!("received termination signal, aborting");
                        break;
                    }
                }
                for _ in 0..concurrency as usize {
                    let handle = tokio::spawn(async { //TODO: graceful handleing
                        // INIT TABLE
                        let table_id = thread_rng().gen_range(TABLE_ID_RANGE);
                        let id = table_id;
                        println!("initializing table={} for customers", id);
                        let res = Client::new()
                            .patch(format!("{}/{}", HOST, "v1/table/") + &id.to_string())
                            .send()
                            .await.expect("failed to init table");
                        let bill_id = match res.status() {
                            StatusCode::OK => {
                                let res = res.json::<TableInitResponse>().await.expect("failed to get response, aborting");
                                println!("table {} initialized successfully, bound to bill id = {}", id, res.bill_id);
                                res.bill_id
                            },
                            StatusCode::BAD_REQUEST => {
                                println!("table {} is already taken, please guide the customers to other tables", id);
                                return;
                            }
                            unexpected => {
                                println!("got unexpected status code, {}", unexpected);
                                panic!("force abort as server is not healthy when initing tables");
                            },
                        };
                        // ADD ITEMS
                        const MENU_SIZE: usize = 5;
                        const MENU_ITEM_ID_RANGE: RangeInclusive<i64> = (1..=5); // there are only 5 menu items, update upper bound to create some BAD_REQUEST possibilities.
                        let menu_ids = (0..MENU_SIZE).into_iter().fold(vec![], |mut acc, _| {
                            acc.push(thread_rng().gen_range(MENU_ITEM_ID_RANGE));
                            acc   
                        });
                        let res = Client::new()
                            .post(format!("{}/{}", HOST, format!("v1/bill/{}/items", bill_id)))
                            .json(&serde_json::json!({
                                "items": menu_ids,
                            }))
                            .send()
                            .await.expect("failed to add items");
                        match res.status() {
                            StatusCode::OK => {
                                println!("Successfully added items to bill id = {}", bill_id);
                            },
                            StatusCode::BAD_REQUEST => {
                                println!("Bad request for bill id = {}, either bill or menu item does not exist", bill_id);
                            }
                            unexpected => {
                                println!("got unexpected status code, {}", unexpected);
                                panic!("force abort as server is not healthy when adding items");
                            },
                        }
                        // LIST ITEMS FOR THIS TABLE
                        let res = Client::new()
                            .get(format!("{}/{}", HOST, format!("v1/bill/{}", bill_id)))
                            .send()
                            .await.expect("failed to list items for bill");
                        let items = match res.status() {
                            StatusCode::OK => {
                                let res = res.json::<GetBillResponse>().await.expect("failed to get response for bill items");
                                let res_items = res.bill.map(|b|b.items).take().unwrap();
                                let items =  res_items.0.into_iter().map(|item| item.id).collect::<Vec<_>>();
                                println!("items for bill = [{}] => {:?}", bill_id, &items);
                                items
                            },
                            StatusCode::BAD_REQUEST => {
                                println!("Bad request");
                                return;
                            },
                            StatusCode::NOT_FOUND => {
                                println!("Resource not found");
                                return;
                            },
                            unexpected => {
                                println!("got unexpected status code, {}", unexpected);
                                panic!("abort due to fail to list items for table");
                            },
                        };
                        // DELETE ITEM
                        let item_id = thread_rng().gen_range(0..items.len()); // if picked id idx = items.len(), we get NOT_FOUND.
                        let res = Client::new()
                            .delete(format!("{}/{}", HOST, format!("v1/bill/{}/item/{}", bill_id, items[item_id])))
                            .send()
                            .await.expect("failed to delete an item");
                        match res.status() {
                            StatusCode::OK => {
                                println!("Successfully removed an item from bill id = {}", bill_id);
                            },
                            StatusCode::BAD_REQUEST => {
                                println!("Bad request");
                            },
                            StatusCode::NOT_FOUND => {
                                println!("Resource not found");
                                panic!("abort due to the item {} to delete from the bill {} does not exist", items[item_id], bill_id);
                            },
                            unexpected => {
                                println!("got unexpected status code, {}", unexpected);
                                panic!("abort due to fail to delete item {} from the bill {}", items[item_id], bill_id);
                            },
                        }
                        // CHECKOUT
                        let res = Client::new()
                            .post(format!("{}/{}", HOST, format!("v1/table/{}", id)))
                            .send()
                            .await.expect(format!("failed to checkout table {}", id).as_str());
                        match res.status() {
                            StatusCode::OK => {
                                println!("Checkout table {} successfully", id);
                            },
                            StatusCode::BAD_REQUEST => {
                                println!("Bad request");
                            },
                            unexpected => {
                                println!("got unexpected status code, {}", unexpected);
                                panic!("abort due to fail to checkout")
                            },
                        };
                        // LIST ALL TABLES
                        let res = Client::new()
                            .get(format!("{}/{}", HOST, format!("v1/tables")))
                            .send()
                            .await.expect("failed to list tables");
                        match res.status() {
                            StatusCode::OK => {
                                let tables = res.json::<GetTablesResponse>().await.expect("failed to get table response").tables;
                                println!("Got tables: {}", tables);
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
                    });   
                }
            }
        }
    };
    Ok(())
}
