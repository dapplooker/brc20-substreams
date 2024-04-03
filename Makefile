# DSN ?= psql://postgres:root@localhost:5432/substreams_e?&schema="brc20"&sslmode=disable
DSN ?= clickhouse://default:@localhost:9000/brc
ENDPOINT ?= mainnet.btc.streamingfast.io:443

.PHONY: build
build:
	cargo build --target wasm32-unknown-unknown --release

.PHONY: protogen
protogen:
	substreams protogen ./substreams.yaml --exclude-paths="sf/substreams,google"

.PHONY: stream_db_out
stream_db_out: build
	substreams run -e $(ENDPOINT) substreams.yaml db_out -s 781930 -t +10000 -o json

.PHONY: create_db
create_db: 
	substreams-sink-sql setup "$(DSN)" sink/substreams.dev.yaml

.PHONY: sink_db_out
sink_db_out: build
	substreams-sink-sql run -e mainnet.btc.streamingfast.io:443 "$(DSN)" sink/substreams.dev.yaml 779830:7799300

.PHONY: create_clickhouse_db
create_clickhouse_db: 
	substreams-sink-sql setup "$(DSN)" sink/substreams-clickhouse.dev.yaml

.PHONY: sink_clickhouse_db_out
sink_clickhouse_db_out: build
	substreams-sink-sql run -e mainnet.btc.streamingfast.io:443 --on-module-hash-mistmatch=warn --flush-interval 1 "$(DSN)" sink/substreams-clickhouse.dev.yaml 779830:7799300 --undo-buffer-size 10