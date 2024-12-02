# bookish-eureka

This is an attempt to create a production-ready application.

## Components
- server: http server to process requests from the client.
- client: cli to interact with the server.

## Requirements
- [ ] The client (the restaurant staff “devices” making the requests) MUST be able to:
  - [x] add one or more items with a table number
  - [x] remove an item for a table
  - [ ] query the items still remaining for a table.
- [x] The application MUST, upon creation request, store
  - [ ] the item, the table number, and how long the item will take to cook.
- [x] The application MUST, upon deletion request, remove a specified item for a specified table number.
- [x] The application MUST, upon query request, show all items for a specified table number.
- [x] The application MUST, upon query request, show a specified item for a specified table number.
- [ ] The application MUST accept at least 10 simultaneous incoming add/remove/query requests.
- [x] The server API MUST fully follow REST API principles and present a set of HTTP endpoints to connect to.

## APIs

### Table
- PATCH /v1/table/{id} : For claiming a table, this creates a new bill for tracking bill items, and bind the bill to the table
- GET /v1/tables : For listing up all tables, and their associated bills.
- POST /v1/table/{id} : For checking out a table at cashier
### Bill
- POST /v1/bill/{id}/items : Add bill associated items to a bill, it's not idempotent so every request creates new items
- DELETE /v1/bill/{id}/item/{item_id} : Remove one specific bill item from a bill, calling it multiple times is safe
- GET /v1/bill/{id} : Get bill items for a bill

## Integration Test

### Prerequisites
- Latest Podman (or Colima, Docker, etc. I use Podman for Desktop on Windows)
- 

## Limitations
- Currently, server address only supports IPv4.