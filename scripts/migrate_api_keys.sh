#!/bin/bash
# API Key Migration Script
# Migrates API keys from old hash format to SHA-256 format

set -e

QDRANT_URL="${QDRANT_URL:-http://localhost:6333}"
COLLECTION="api_keys"

echo "=========================================="
echo "API Key Migration Script"
echo "=========================================="
echo "Qdrant URL: $QDRANT_URL"
echo "Collection: $COLLECTION"
echo ""
echo "⚠️  WARNING: This is a BREAKING CHANGE"
echo "All existing API keys will be invalidated."
echo "You must re-issue all API keys after migration."
echo ""
read -p "Continue? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Migration cancelled."
    exit 0
fi

echo ""
echo "Step 1: Backing up existing collection..."
BACKUP_FILE="api_keys_backup_$(date +%Y%m%d_%H%M%S).json"

curl -s "$QDRANT_URL/collections/$COLLECTION/points/scroll" \
    -H "Content-Type: application/json" \
    -d '{"limit": 1000, "with_payload": true, "with_vector": true}' \
    > "$BACKUP_FILE"

if [ $? -eq 0 ]; then
    echo "✓ Backup saved to: $BACKUP_FILE"
else
    echo "✗ Backup failed. Aborting."
    exit 1
fi

echo ""
echo "Step 2: Deleting old collection..."
curl -X DELETE "$QDRANT_URL/collections/$COLLECTION" || true

echo ""
echo "Step 3: Creating new collection with updated schema..."
curl -X PUT "$QDRANT_URL/collections/$COLLECTION" \
    -H "Content-Type: application/json" \
    -d '{
        "vectors": {
            "size": 1,
            "distance": "Cosine"
        }
    }'

echo ""
echo "=========================================="
echo "Migration Complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. Backup file saved: $BACKUP_FILE"
echo "2. Re-issue all API keys using: POST /v1/admin/keys"
echo "3. Update your applications with new keys"
echo ""
echo "Note: Old keys are stored in the backup file for reference."
