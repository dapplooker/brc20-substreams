-- Create schema
CREATE DATABASE IF NOT EXISTS brc20;

-- Create cursors table
CREATE TABLE IF NOT EXISTS brc20.cursors (
    id        String NOT NULL,
    cursor    String,
    block_num Int64,
    block_id  String
) ENGINE = ReplacingMergeTree()
    PRIMARY KEY (id)
    ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20.Deploy (
    id          String,
    deployer    String,
    block       String,
    timestamp   Int64,
    token       String
) ENGINE = ReplacingMergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20.Token (
    id           String,
    deployment   FixedString(70),
    decimals     Int32,
    max_supply   String,
    symbol       FixedString(70),
    mintlimit    String,
    timestamp    Int64
) ENGINE = ReplacingMergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20.Mint (
    id       String,
    token    FixedString(70),
    to       FixedString(70),
    amount   String,
    timestamp   Int64
) ENGINE = MergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20.Transfer (
    id         String,
    token      FixedString(70),
    to         FixedString(70),
    from       FixedString(70),
    amount     Int64,
    timestamp   Int64
) ENGINE = MergeTree()
ORDER BY (id);


CREATE TABLE IF NOT EXISTS brc20.Balance (
    id            String,
    token         FixedString(70),
    account       FixedString(70),
    balance       Int64,
    transferable  String,
    timestamp   Int64
) ENGINE = ReplacingMergeTree()
ORDER BY (id);


CREATE TABLE IF NOT EXISTS brc20.Account (
    address    FixedString(70)
) ENGINE = ReplacingMergeTree()
ORDER BY (address);
