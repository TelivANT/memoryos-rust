#!/bin/bash
# Sync code to remote server (using tar+scp)

set -e

SERVER="root@104.194.91.83"
PORT="26974"
REMOTE_DIR="/opt/memoryos"

echo "🔄 Syncing code to remote server..."

# Create tarball
tar czf /tmp/memoryos-sync.tar.gz \
  --exclude='target' \
  --exclude='.git' \
  --exclude='*.log' \
  --exclude='.env' \
  .

# Upload
scp -P $PORT /tmp/memoryos-sync.tar.gz $SERVER:/tmp/

# Extract on server
ssh -p $PORT $SERVER "mkdir -p $REMOTE_DIR && cd $REMOTE_DIR && tar xzf /tmp/memoryos-sync.tar.gz && rm /tmp/memoryos-sync.tar.gz"

# Cleanup
rm /tmp/memoryos-sync.tar.gz

echo "✅ Code synced to $SERVER:$REMOTE_DIR"
echo ""
echo "📝 Next steps:"
echo "  ssh -p $PORT $SERVER"
echo "  cd $REMOTE_DIR"
echo "  cargo build --release"
echo "  docker-compose up -d"
