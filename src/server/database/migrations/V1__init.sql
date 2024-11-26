-- table in the restaurant
CREATE TABLE IF NOT EXISTS "table" (
    id smallserial PRIMARY KEY,
    bill_id bigint
);


-- bill for tables
CREATE TABLE IF NOT EXISTS bill (
     id bigserial PRIMARY KEY,
     table_id smallint NOT NULL, -- index
     created_at timestamptz NOT NULL, -- index
     updated_at timestamptz,
     checkout_at timestamptz, -- index
     CONSTRAINT fk_table_id FOREIGN KEY(table_id) REFERENCES "table"(id) ON DELETE CASCADE
);

-- for querying tables with item list
CREATE INDEX IF NOT EXISTS table_list_idx on bill(table_id, created_at, checkout_at);

-- menu item for the restaurant
CREATE TABLE IF NOT EXISTS menu_item (
    id serial PRIMARY KEY,
    name varchar(32) NOT NULL,
    category varchar(4) NOT NULL
);

-- menu item for bills
CREATE TABLE IF NOT EXISTS bill_item (
    id bigserial PRIMARY KEY,
    bill_id bigserial NOT NULL, -- index
    menu_item_id integer NOT NULL, -- index
    state varchar(16), -- index,
    CONSTRAINT fk_bill_id FOREIGN KEY(bill_id) REFERENCES bill(id) ON DELETE CASCADE,
    CONSTRAINT fk_menu_item_id FOREIGN KEY(menu_item_id) REFERENCES menu_item(id)
);

-- for joining item list from bill table, including menu_item_id for covering index, so that we can do aggregation quicker
CREATE INDEX IF NOT EXISTS item_list_idx on bill_item(bill_id, menu_item_id, state);


-- init "table" with some tables 
INSERT INTO "table"(id)
VALUES
    (1),
    (2),
    (3),
    (4),
    (5);

INSERT INTO menu_item(id, name, category)
VALUES
    (1, 'Fried chicken', 'A'),
    (2, 'French fries', 'A');