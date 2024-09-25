DROP TABLE IF EXISTS products;

CREATE TABLE products (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    avg_price DOUBLE PRECISION NOT NULL DEFAULT 5.0
);