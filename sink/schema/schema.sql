CREATE SCHEMA brc20;

CREATE TABLE brc20.cursors (
    id         text not null constraint cursor_pk primary key,
    cursor     text,
    block_num  bigint,
    block_id   text
);

CREATE TABLE brc20.Deploy (
    id         text  not null constraint deploy_pk primary key,
    deployer   text,
    block      text,
    "timestamp"  text,
    token      text
);

CREATE TABLE brc20.Token (
    id         text not null constraint tokens_pk primary key,
    deployment   text,
    decimals      text,
    max_supply  text,
    symbol      text,
    mintlimit   text
);

CREATE TABLE brc20.Mint (
    id         text not null constraint mint_pk primary key,
    token   text,
    "to"      text,
    amount  text
);

CREATE TABLE brc20.Transfer (
    id         text not null constraint transfer_pk primary key,
    token   text,
    "to"      text,
    "from"  text,
    amount  text
);

CREATE TABLE brc20.Balance (
    id         text not null constraint account_pk primary key,
    token      text,
    account      text,
    balance  text,
    transferable  text
);

CREATE TABLE brc20.Account (
    id         text not null constraint accounts_pk primary key,
    address      text
);
