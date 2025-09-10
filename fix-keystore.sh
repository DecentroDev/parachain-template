#!/bin/bash
# Fix the keystore issue by inserting keys for offchain workers

echo "🔑 Adding keys to keystore for DAO offchain workers..."

# Insert Alice's key for offchain worker signing
# The DAO pallet uses sr25519 keys for signing transactions
curl -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["dao", "//Alice", "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]}' \
     http://localhost:9944

echo ""
echo "✅ Alice's key inserted for DAO offchain workers"
echo ""

# You can also insert custom keys if needed:
echo "🔧 To insert custom keys, use:"
echo 'curl -H "Content-Type: application/json" \'
echo '     -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"dao\", \"//YourSeed\", \"YOUR_HEX_KEY\"]}" \'
echo '     http://localhost:9944'





