# bookish-eureka

This is an attempt to create a production-ready application.

## Components
- server: http server to process requests from the client.
- client: cli to interact with the server.

## APIs

### Table
- PATCH /v1/table/{id} : For claiming a table, this creates a new bill for tracking bill items, and bind the bill to the table
- GET /v1/tables : For listing up all tables, and their associated bills.
- POST /v1/table/{id} : For checking out a table at cashier
### Bill
- POST /v1/bill/{id}/items : Add bill associated items to a bill, it's not idempotent so every request creates new items
- DELETE /v1/bill/{id}/item/{item_id} : Remove one specific bill item from a bill, calling it multiple times is safe
- GET /v1/bill/{id} : Get bill items for a bill

## Usage

### Server
```bash
$ cargo run --bin server
```
### Client
```bash
$ cargo run --features="build-client" --bin client
```
### Integration test

#### Prerequisites
- OCI Container runtimes like Podman, Colima, or Docker.
  - One might need to install plugin for compose subcommand.

#### Steps (Using Podman as an example)

1. Bring up the backend service (server, database)
```bash
$ podman compose up [--build]
```
2. After it's ready, run database migration(s)
```bash
$ cargo run --bin refinery_migration
```
3. Run client command with desired simulated concurrency
```bash
$ cargo run --features="build-client" --bin client test CONCURRENCY
```

## CI
- use tarpaulin to generate code coverage and paste comment to pull requests.

## Requirements
- [x] The client (the restaurant staff “devices” making the requests) MUST be able to:
  - [x] add one or more items with a table number
  - [x] remove an item for a table
  - [x] query the items still remaining for a table.
- [x] The application MUST, upon creation request, store
  - [x] the item, the table number, and how long the item will take to cook.
- [x] The application MUST, upon deletion request, remove a specified item for a specified table number.
- [x] The application MUST, upon query request, show all items for a specified table number.
- [x] The application MUST, upon query request, show a specified item for a specified table number.
- [x] The application MUST accept at least 10 simultaneous incoming add/remove/query requests.
- [x] The server API MUST fully follow REST API principles and present a set of HTTP endpoints to connect to.

## Limitations
- Currently, server address only supports IPv4.

## MSRV
Unknown. Used cargo-msrv it failed with below messages:
```
Result:
   Considered (min … max):   Rust 1.56.1 … Rust 1.83.0
   Search method:            bisect
   MSRV:                     N/A
   Target:                   x86_64-pc-windows-msvc

Unable to find a Minimum Supported Rust Version (MSRV).
```
But I build this project without issues on below toolchain version:
```
rustc 1.84.0-nightly (b3f75cc87 2024-11-02)
```

## Future works
- Authentication : Guard the API endpoints, e.g. Oauth2.
- Observability : Monitoring for the service & database, e.g. opentelemetry + tracing + Prometheus + Grafana.
- Refactor : Create service/repository layers instead of clogging request handler methods with all kinds of operations.
