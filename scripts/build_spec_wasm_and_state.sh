#!/usr/bin/env sh

./target/release/crust-collator build-spec --raw --disable-default-bootnode --chain 2008 > ./crust-collator/res/2008.json
./target/release/crust-collator build-spec --raw --disable-default-bootnode --chain 2012 > ./crust-collator/res/2012.json
./target/release/crust-collator build-spec --raw --disable-default-bootnode --chain staging > ./crust-collator/res/staging.json

# sed command is slightly different between MacOS and Linux 
if [ "$(uname)" = "Darwin" ]; then
    # MacOS - sed in MacOS need an additional argument to specify the extension
    SED_CMD="sed -i ''" 
else
    # Linux
    SED_CMD="sed -i"
fi

$SED_CMD "s/\"protocolId\": null/\"protocolId\": \"2008\"/g" ./crust-collator/res/2008.json
$SED_CMD "s/\"name\": \"Local Testnet\"/\"name\": \"2008\"/g" ./crust-collator/res/2008.json
$SED_CMD "s/\"id\": \"local_testnet\"/\"id\": \"2008\"/g" ./crust-collator/res/2008.json
$SED_CMD "s/\"properties\": null,/\"properties\": {\"ss58Format\": 42, \"tokenDecimals\": 12, \"tokenSymbol\": \"2008\"},/g" ./crust-collator/res/2008.json

$SED_CMD "s/\"protocolId\": null/\"protocolId\": \"2012\"/g" ./crust-collator/res/2012.json
$SED_CMD "s/\"name\": \"Local Testnet\"/\"name\": \"2012\"/g" ./crust-collator/res/2012.json
$SED_CMD "s/\"id\": \"local_testnet\"/\"id\": \"2012\"/g" ./crust-collator/res/2012.json
$SED_CMD "s/\"properties\": null,/\"properties\": {\"ss58Format\": 42, \"tokenDecimals\": 12, \"tokenSymbol\": \"2012\"},/g" ./crust-collator/res/2012.json

$SED_CMD "s/\"protocolId\": null/\"protocolId\": \"2012\"/g" ./crust-collator/res/staging.json
$SED_CMD "s/\"name\": \"Local Testnet\"/\"name\": \"2012\"/g" ./crust-collator/res/staging.json
$SED_CMD "s/\"id\": \"local_testnet\"/\"id\": \"2012\"/g" ./crust-collator/res/staging.json
$SED_CMD "s/\"properties\": null,/\"properties\": {\"ss58Format\": 42, \"tokenDecimals\": 12, \"tokenSymbol\": \"2012\"},/g" ./crust-collator/res/staging.json

./target/release/crust-collator export-genesis-state --chain ./crust-collator/res/2012.json > crust-collator-state-2012
./target/release/crust-collator export-genesis-state --chain ./crust-collator/res/2008.json > crust-collator-state-2008
./target/release/crust-collator export-genesis-state --chain ./crust-collator/res/staging.json > crust-collator-state-staging

./target/release/crust-collator export-genesis-wasm --chain ./crust-collator/res/2012.json > crust-collator-wasm-2012
./target/release/crust-collator export-genesis-wasm --chain ./crust-collator/res/2008.json > crust-collator-wasm-2008
./target/release/crust-collator export-genesis-wasm --chain ./crust-collator/res/staging.json > crust-collator-wasm-staging