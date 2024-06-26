mod brc20;
mod btc_utils;
mod ord;
mod pb;
mod tables_utils;

use std::str::FromStr;

use anyhow::Result;
use brc20::Brc20Event;
use btc_utils::{btc_to_sats, parse_inscriptions};
use pb::btc::brc20::v1::{
    Brc20Events, Deploy, ExecutedTransfer, InscribedTransfer, InscribedTransferLocation, Mint,
    Token,
};
use pb::sf::bitcoin::r#type::v1 as btc;
use substreams::pb::substreams::store_delta::Operation;
use substreams::pb::substreams::Clock;
use substreams::scalar::BigInt;
use substreams::store::{
    DeltaBigInt, Deltas, StoreAdd, StoreAddBigInt, StoreGet, StoreGetProto, StoreNew, StoreSet,
    StoreSetProto,
};
use substreams_database_change::tables::Tables;
use substreams_database_change::pb::database::{ DatabaseChanges};

struct Location {
    pub utxo: String,
    pub offset: u64,
    pub utxo_amount: u64,
}

#[substreams::handlers::map]
fn map_brc20_events(block: btc::Block) -> Result<Brc20Events, substreams::errors::Error> {
    let events = block
        .tx
        .into_iter()
        // Filter if tx data contains "text/plain;charset=utf-8" inscriptions
        .filter(|tx| {
            tx.hex
                .contains("746578742f706c61696e3b636861727365743d7574662d38")
        })
        .flat_map(|tx| {
           
            let txid = tx.txid.clone();
                        match parse_inscriptions(&tx) {
                Ok(inscriptions) => inscriptions
                    .into_iter()
                    .filter(|inscription| {
                        match inscription
                            .content_type()
                            .map(|ctype| ctype.split(";"))
                            .and_then(|mut parts| parts.next())
                        {
                            Some(content_type) => {
                                content_type == "text/plain" || content_type == "application/json"
                            }
                            None => false,
                        }
                    })
                    .filter_map(|inscription| {
                        let (vout, offset) = tx.nth_sat_utxo(inscription.pointer().unwrap_or(0))?;
                        Some((
                            Location {
                                utxo: format!("{}:{}", tx.txid, offset),
                                offset,
                                utxo_amount: btc_to_sats(vout.value),
                            },
                            vout.address(),
                            inscription,
                        ))
                    })
                    .collect(),
                Err(err) => {
                    substreams::log::info!("Error parsing inscriptions in tx {}: {}", txid, err);
                    vec![]
                }
            }
        })
        .filter_map(|(location, address, inscription)| {
            let content = if let Ok(content) =
                String::from_utf8(inscription.body().unwrap_or_default().to_vec())
            {
                content
            } else {
                return None;
            };

            match serde_json::from_str::<Brc20Event>(&content) {
                Ok(event) if event.valid() => Some((location, address, event)),
                Ok(_) => None,
                Err(err) => {
                    substreams::log::info!(
                        "Error parsing inscription content {}: {}",
                        location.utxo,
                        err
                    );
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(Brc20Events {
        deploys: events
            .iter()
            .filter_map(|(_, address, event)| match (address, event) {
                (Some(address), Brc20Event::Deploy(deploy)) => Some(Deploy {
                    id: "".into(),
                    symbol: deploy.tick(),
                    max_supply: deploy.max.to_string(),
                    mint_limit: deploy.lim().to_string(),
                    decimals: deploy.dec(),
                    deployer: address.clone(),
                }),
                _ => None,
            })
            .collect(),
        mints: events
            .iter()
            .filter_map(|(_, address, event)| match (address, event) {
                (Some(address), Brc20Event::Mint(mint)) => Some(Mint {
                    id: "".into(),
                    token: mint.tick(),
                    to: address.into(),
                    amount: mint.amt.to_string(),
                }),
                _ => None,
            })
            .collect(),
        inscribed_transfers: events
            .iter()
            .filter_map(|(location, address, event)| match (address, event) {
                (Some(address), Brc20Event::Transfer(transfer)) => Some(InscribedTransfer {
                    id: location.utxo.split(':').next().unwrap_or("").to_string(),
                    token: transfer.tick(),
                    // to: "".into(),
                    from: address.into(),
                    amount: transfer.amt.to_string(),
                    utxo: location.utxo.clone(),
                    offset: location.offset,
                    utxo_amount: location.utxo_amount,
                }),
                _ => None,
            })
            .collect(),
        executed_transfers: vec![],
    })
}

#[substreams::handlers::store]
fn store_inscribed_transfers(events: Brc20Events, store: StoreSetProto<InscribedTransferLocation>) {
    events.inscribed_transfers.iter().for_each(|transfer| {
        store.set(
            0,
            transfer.utxo.clone(),
            &InscribedTransferLocation {
                id: transfer.id.clone(),
                token: transfer.token.clone(),
                from: transfer.from.clone(),
                amount: transfer.amount.clone(),
                offset: transfer.offset.clone(),
                utxo_amount: transfer.utxo_amount.clone(),
            },
        );
    });
}

#[substreams::handlers::store]
fn store_tokens(events: Brc20Events, store: StoreSetProto<Token>) {
    events.deploys.iter().for_each(|deploy| {
        store.set(
            0,
            deploy.symbol.clone(),
            &Token {
                id: deploy.id.clone(),
                symbol: deploy.symbol.clone(),
                max_supply: deploy.max_supply.clone(),
                mint_limit: deploy.mint_limit.clone(),
                decimals: deploy.decimals.clone(),
                deployer: deploy.deployer.clone(),
            },
        );
    });
}

#[substreams::handlers::store]
fn store_balances(events: Brc20Events, store: StoreAddBigInt) {
    // On mints, we add the amount to the receiver's balance
    events.mints.iter().for_each(|mint| {
        store.add(
            0,
            format!("{}:{}", mint.token, mint.to),
            BigInt::from_str(&mint.amount).expect("Amount should be valid integer"),
        );
    });

    // On inscribed transfers, we subtract the amount from the sender's balance.
    // Note: The sender's transferable balance is increased in the
    // `store_transferable_balance` store module
    events.inscribed_transfers.iter().for_each(|transfer| {
        store.add(
            0,
            format!("{}:{}", transfer.token, transfer.from),
            BigInt::from_str(&transfer.amount)
                .expect("Amount should be valid integer")
                .neg(),
        );
    });

    // On executed transfers, we add the amount to the receiver's balance
    events.executed_transfers.iter().for_each(|transfer| {
        store.add(
            0,
            format!("{}:{}", transfer.token, transfer.to),
            BigInt::from_str(&transfer.amount).expect("Amount should be valid integer"),
        );
    });
}

#[substreams::handlers::store]
fn store_transferable_balances(events: Brc20Events, store: StoreAddBigInt) {
    // On inscribed transfers, we add the amount to the sender's transferable balance
    events.inscribed_transfers.iter().for_each(|transfer| {
        store.add(
            0,
            format!("{}:{}", transfer.token, transfer.from),
            BigInt::from_str(&transfer.amount).expect("Amount should be valid integer"),
        );
    });

    // On executed transfers, we subtract the amount from the sender's transferable balance
    events.executed_transfers.iter().for_each(|transfer| {
        store.add(
            0,
            format!("{}:{}", transfer.token, transfer.from),
            BigInt::from_str(&transfer.amount)
                .expect("Amount should be valid integer")
                .neg(),
        );
    });
}

#[substreams::handlers::map]
fn map_resolve_transfers(
    block: btc::Block,
    events: Brc20Events,
    transfer_store: StoreGetProto<InscribedTransferLocation>,
    token_store: StoreGetProto<Token>,
) -> Result<Brc20Events, substreams::errors::Error> {
    let executed_transfers =
        block
            .tx
            .into_iter()
            .filter_map(|tx| {
                // Note: Without tracking UTXO values, we can only reliably resolve transfers where the
                // inscribed sat is held by the first input UTXO of the transaction
                if let Some(inscribed_transfer_loc) =
                    transfer_store.get_at(0, format!("{}:{}", tx.vin[0].txid, tx.vin[0].vout))
                {
                    let (vout, _) = tx.nth_sat_utxo(inscribed_transfer_loc.offset)?;
                    
                    Some(ExecutedTransfer {
                        id: inscribed_transfer_loc.id,
                        token: inscribed_transfer_loc.token,
                        from: inscribed_transfer_loc.from,
                        to: vout.address()?,
                        amount: inscribed_transfer_loc.amount,
                    })
                } else {
                    // Log that we could not resolve transfer
                    if let Some(inscribed_transfer_loc) = tx.vin.iter().find_map(|vin| {
                        transfer_store.get_at(0, format!("{}:{}", vin.txid, vin.vout))
                    }) {
                        substreams::log::info!(
                            "Could not resolve inscribed transfer {}",
                            inscribed_transfer_loc.id
                        );
                    }
                    None
                }
            })
            .collect::<Vec<_>>();

    Ok(Brc20Events {
        executed_transfers,
        mints: events
            .mints
            .into_iter()
            .filter(|mint| match token_store.get_at(0, mint.token.clone()) {
                Some(token) => {
                    BigInt::from_str(&mint.amount).unwrap()
                        < BigInt::from_str(&token.mint_limit).unwrap()
                }
                None => false,
            })
            .collect(),
        ..events
    })
}

#[substreams::handlers::map]
fn db_out(
    clock: Clock,
    events: Brc20Events,
    balances_store: Deltas<DeltaBigInt>,
    transferable_balances_store: Deltas<DeltaBigInt>,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut tables: Tables = Tables::new();

    events.deploys.iter().for_each(|deploy| {
        tables
        .create_row("Deploy", clock
        .timestamp
        .as_ref()
        .map(|t| t.seconds)
        .unwrap_or_default().to_string())
        .set("token", deploy.symbol.clone())
        .set("deployer", deploy.deployer.clone())
        .set("block", clock.number.clone())
        .set(
            "timestamp",
            clock
                .timestamp
                .as_ref()
                .map(|t| t.seconds)
                .unwrap_or_default(),
        );

    tables
        .create_row("Token", deploy.symbol.clone().to_string())
        .set("symbol", deploy.symbol.clone().to_string())
        .set("max_supply", &deploy.max_supply.to_string())  
        .set("mintlimit", &deploy.mint_limit.to_string())
        .set("decimals", deploy.decimals.clone().to_string())
        .set(
            "timestamp",
            clock
                .timestamp
                .as_ref()
                .map(|t| t.seconds)
                .unwrap_or_default(),
        )
        .set("deployment", deploy.id.clone().to_string());
    });

    events.mints.iter().for_each(|mint| {
        tables
            .create_row("Mint", mint.id.clone())
            .set("token", mint.token.clone())
            .set("to", mint.to.clone())
            .set(
                "timestamp",
                clock
                    .timestamp
                    .as_ref()
                    .map(|t| t.seconds)
                    .unwrap_or_default(),
            )
            .set("amount", &mint.amount);
    });

    events.executed_transfers.iter().for_each(|transfer| {
        let amount_as_int: i64 = transfer.amount.parse().unwrap_or(0); 
        tables
            .create_row("Transfer", transfer.id.clone())
            .set("token", transfer.token.clone())
            .set("from", transfer.from.clone())
            .set("to", transfer.to.clone())
            .set(
                "timestamp",
                clock
                    .timestamp
                    .as_ref()
                    .map(|t| t.seconds)
                    .unwrap_or_default(),
            )
            .set("amount", amount_as_int);
    });

    balances_store
        .deltas
        .iter()
        .for_each(|delta| match delta.operation {
            Operation::Create => {
                let (token, account) = {
                    let mut parts = delta.key.split(':');
                    (
                        parts
                            .next()
                            .expect("Balance store key should be `{SYMBOL}:{ACCOUNT}`"),
                        parts
                            .next()
                            .expect("Balance store key should be `{SYMBOL}:{ACCOUNT}`"),
                    )
                };

                tables
                    .create_row("Balance", delta.key.clone())
                    .set("account", account.to_string())
                    .set("token", token.to_string())
                    .set("balance", &delta.new_value)
                    .set(
                        "timestamp",
                        clock
                            .timestamp
                            .as_ref()
                            .map(|t| t.seconds)
                            .unwrap_or_default(),
                    )
                    .set::<String>("transferable", "0".to_string().into());

                // tables.create_row("Account", account);
                tables
            .create_row("Account",account)
            .set("address", account.clone().to_string());
            }
            Operation::Update => {
                // tables
                //     .update_row("Balance", delta.key.clone())
                //     .set("balance", &delta.new_value.to_string());
            }
            _ => (),
        });

    transferable_balances_store.deltas.iter().for_each(|delta| {
        // Note: No need to check operation since the AccountBalance row should have been created
        // when the `balances_store` had a `Create` operation for the same key.
        // This is because an account can only have a transferable balance if it has a balance
        // in the first place, which is created when the account is the recipient of either a Mint
        // or a Transfer event.
        // tables
        //     .update_row("Balance", delta.key.clone())
        //     .set("transferable", &delta.new_value.to_string());
    });

    Ok(tables.to_database_changes())
}

