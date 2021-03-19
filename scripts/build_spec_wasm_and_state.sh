#!/usr/bin/env sh

./target/release/crust-collator build-spec --raw --disable-default-bootnode --chain 6666 > ./crust-collator/res/6666.json
./target/release/crust-collator build-spec --raw --disable-default-bootnode --chain 7777 > ./crust-collator/res/7777.json

sed -i "" "s/\"protocolId\": null/\"protocolId\": \"6666\"/g" ./crust-collator/res/6666.json
sed -i "" "s/\"name\": \"Local Testnet\"/\"name\": \"6666\"/g" ./crust-collator/res/6666.json
sed -i "" "s/\"id\": \"local_testnet\"/\"id\": \"6666\"/g" ./crust-collator/res/6666.json
sed -i "" "s/\"properties\": null,/\"properties\": {\"ss58Format\": 42, \"tokenDecimals\": 12, \"tokenSymbol\": \"6666\"},/g" ./crust-collator/res/6666.json

sed -i "" "s/\"protocolId\": null/\"protocolId\": \"7777\"/g" ./crust-collator/res/7777.json
sed -i "" "s/\"name\": \"Local Testnet\"/\"name\": \"7777\"/g" ./crust-collator/res/7777.json
sed -i "" "s/\"id\": \"local_testnet\"/\"id\": \"7777\"/g" ./crust-collator/res/7777.json
sed -i "" "s/\"properties\": null,/\"properties\": {\"ss58Format\": 42, \"tokenDecimals\": 12, \"tokenSymbol\": \"7777\"},/g" ./crust-collator/res/7777.json

./target/release/crust-collator export-genesis-state --chain ./crust-collator/res/7777.json  > crust-collator-state-7777
./target/release/crust-collator export-genesis-state --chain ./crust-collator/res/6666.json  > crust-collator-state-6666
./target/release/crust-collator export-genesis-wasm --chain ./crust-collator/res/6666.json > crust-collator-wasm-7777
./target/release/crust-collator export-genesis-wasm --chain ./crust-collator/res/6666.json > crust-collator-wasm-6666