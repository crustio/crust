curl --location --request POST 'http://localhost:20022/' \
--header 'Content-Type: application/json' \
--data-raw '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "aura",
      "list cloth dddde salad session gain accuse skull tongue toss regular guide",
      "0x7a6a226782a4cf5712f9er34f3cc64304f3c9af58b82f1dd2a4f09c48278ae65"
    ]
}'