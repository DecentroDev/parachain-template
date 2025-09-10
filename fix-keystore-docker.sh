#!/bin/bash
# Script to add Alice's key to the keystore for DAO offchain workers

echo "🔑 Adding Alice's key to the keystore for DAO offchain workers..."

# Wait a moment for the node to be ready
sleep 2

# Add Alice's key for DAO signing
curl -s -X POST -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["dao", "//Alice", "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]}' \
     http://localhost:9944 > /dev/null && echo "✅ Alice's key added for DAO signing" || echo "⚠️  Key insertion failed"

echo ""
echo "🎉 Keystore fix complete! The DAO offchain workers should now be able to sign transactions."
