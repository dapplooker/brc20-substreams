-- Create schema
CREATE DATABASE IF NOT EXISTS brc20_token;

-- Create cursors table
CREATE TABLE IF NOT EXISTS brc20_token.cursors (
    id        String NOT NULL,
    cursor    String,
    block_num Int64,
    block_id  String
) ENGINE = ReplacingMergeTree()
    PRIMARY KEY (id)
    ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20_token.Deploy (
    id          String,
    deployer    String,
    block       String,
    timestamp   String,
    token       String
) ENGINE = ReplacingMergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20_token.Token (
    id           String,
    deployment   FixedString(70),
    decimals     Int32,
    max_supply   String,
    symbol       FixedString(70),
    mintlimit    String
) ENGINE = ReplacingMergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20_token.Mint (
    id       String,
    token    FixedString(70),
    to       FixedString(70),
    amount   String
) ENGINE = MergeTree()
ORDER BY (id);

CREATE TABLE IF NOT EXISTS brc20_token.Transfer (
    id         String,
    token      FixedString(70),
    to         FixedString(70),
    from       FixedString(70),
    amount     String
) ENGINE = MergeTree()
ORDER BY (id);


CREATE TABLE IF NOT EXISTS brc20_token.Balance (
    id            String,
    token         FixedString(70),
    account       FixedString(70),
    balance       String,
    transferable  String
) ENGINE = ReplacingMergeTree()
ORDER BY (id);


CREATE TABLE IF NOT EXISTS brc20_token.Account (
    address    FixedString(70)
) ENGINE = ReplacingMergeTree()
ORDER BY (address);
