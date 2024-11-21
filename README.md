# bookish-eureka

This is an attempt to create a production-ready application.

## Components
- server: http server to process requests from the client.
- client: cli to interact with the server.

## Features
- Oauth2 for client authentication
## Requirements
- [ ] The client (the restaurant staff “devices” making the requests) MUST be able to:
  - [ ] add one or more items with a table number
  - [ ] remove an item for a table
  - [ ] query the items still remaining for a table.
- [ ] The application MUST, upon creation request, store
  - [ ] the item, the table number, and how long the item will take to cook.
- [ ] The application MUST, upon deletion request, remove a specified item for a specified table number.
- [ ] The application MUST, upon query request, show all items for a specified table number.
- [ ] The application MUST, upon query request, show a specified item for a specified table number.
- [ ] The application MUST accept at least 10 simultaneous incoming add/remove/query requests.
- [ ] The server API MUST fully follow REST API principles and present a set of HTTP endpoints to connect to.

## Usage

### Server
```bash
$ [APP_ENV=<stg|prod>] ./server
```
### Client

#### Create order
```bash
$ ./client create [<order name> <item>.. ]
```
#### Delete order
```bash
$ ./client create
```

## Testing

### Integration Test

### Unit Test

### Local test
Some handy curl cmds.
```curl
Create Bill

curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"table_id": 1, "customer_count": 4}' \
  http://localhost:8080/v1/bills
  
  
Get a Bill
curl localhost:8080/v1/bill/{id}

Get Bills
curl localhost:8080/v1/bills
```

## Limitations
- Currently, server address only supports IPv4.