#!/bin/bash
# API Key Migration Script
# Migrates from plaintext to SHA-256 hashed storage
# Version: 0.2.0 -> 0.2.1

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  API Key Migration Script${NC}"
echo -e "${GREEN}  v0.2.0 -> v0.2.1${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Configuration
QDRANT_URL="${QDRANT_URL:-http://localhost:6334}"
COLLECTION="api_keys"
BACKUP_FILE="api_keys_backup_$(date +%Y%m%d_%H%M%S).json"

echo -e "${YELLOW}⚠️  WARNING: This is a BREAKING CHANGE${NC}"
echo -e "${YELLOW}   All existing API keys will be re-hashed${NC}"
echo -e "${YELLOW}   Backup will be saved to: $BACKUP_FILE${NC}"
echo ""
read -p "Continue? (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Migration cancelled."
    exit 0
fi

# Check if Qdrant is accessible
echo ""
echo -e "${YELLOW}Checking Qdrant connection...${NC}"
if ! curl -sf "$QDRANT_URL/health" > /dev/null; then
    echo -e "${RED}Error: Cannot connect to Qdrant at $QDRANT_URL${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Qdrant is accessible${NC}"

# Backup existing keys
echo ""
echo -e "${YELLOW}Backing up existing API keys...${NC}"
curl -sf "$QDRANT_URL/collections/$COLLECTION/points/scroll" \
    -H "Content-Type: application/json" \
    -d '{"limit": 1000, "with_payload": true, "with_vector": false}' \
    > "$BACKUP_FILE"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Backup saved to $BACKUP_FILE${NC}"
    BACKUP_COUNT=$(jq '.result.points | length' "$BACKUP_FILE")
    echo -e "${GREEN}  Found $BACKUP_COUNT API keys${NC}"
else
    echo -e "${RED}Error: Failed to backup API keys${NC}"
    exit 1
fi

# Check if any keys exist
if [ "$BACKUP_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}No API keys found. Migration not needed.${NC}"
    exit 0
fi

# Migration warning
echo ""
echo -e "${RED}========================================${NC}"
echo -e "${RED}  IMPORTANT MIGRATION NOTICE${NC}"
echo -e "${RED}========================================${NC}"
echo -e "${YELLOW}After migration, you CANNOT recover the original API keys.${NC}"
echo -e "${YELLOW}The plaintext 'api_key' field will be replaced with 'key_hash'.${NC}"
echo ""
echo -e "${YELLOW}You have two options:${NC}"
echo -e "${YELLOW}1. Re-issue new API keys to all users${NC}"
echo -e "${YELLOW}2. Keep the backup file and manually map old keys to new hashes${NC}"
echo ""
read -p "I understand and want to proceed (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Migration cancelled."
    exit 0
fi

# Delete old collection
echo ""
echo -e "${YELLOW}Deleting old collection...${NC}"
curl -sf -X DELETE "$QDRANT_URL/collections/$COLLECTION"
echo -e "${GREEN}✓ Old collection deleted${NC}"

# Recreate collection
echo ""
echo -e "${YELLOW}Recreating collection with new schema...${NC}"
curl -sf -X PUT "$QDRANT_URL/collections/$COLLECTION" \
    -H "Content-Type: application/json" \
    -d '{
        "vectors": {
            "size": 1,
            "distance": "Cosine"
        }
    }'
echo -e "${GREEN}✓ Collection recreated${NC}"

# Migration complete
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Migration Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo -e "1. Restart MemoryOS Gateway"
echo -e "2. Re-issue API keys to all users using:"
echo -e "   ${GREEN}curl -X POST http://localhost:8080/v1/admin/keys \\${NC}"
echo -e "   ${GREEN}  -H 'Authorization: Bearer <ADMIN_KEY>' \\${NC}"
echo -e "   ${GREEN}  -H 'Content-Type: application/json' \\${NC}"
echo -e "   ${GREEN}  -d '{\"api_key\":\"new_key\",\"user_id\":\"user1\",\"description\":\"...\",\"permissions\":[]}'${NC}"
echo ""
echo -e "${YELLOW}Backup file: $BACKUP_FILE${NC}"
echo -e "${YELLOW}Keep this file safe for reference!${NC}"
echo ""
