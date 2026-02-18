# MemoryOS Python SDK

Python client for MemoryOS - High-performance AI Agent memory management system.

## Installation

```bash
pip install memoryos-sdk
```

## Quick Start

```python
from memoryos import MemoryOS

# Initialize client
client = MemoryOS(base_url="http://localhost:8080")

# Add memory
client.add_memory(
    user_id="user_123",
    role="user",
    content="I love Rust programming"
)

# Retrieve context
context = client.retrieve_context(
    user_id="user_123",
    query="What do I like?"
)
print(context)

# Chat with memory
response = client.chat(
    messages=[
        {"role": "user", "content": "What do you know about me?"}
    ],
    model="gpt-4o"
)
print(response)

# Get memory history
history = client.get_history(memory_id="mem_123")
print(history)

# Health check
status = client.health_check()
print(status)
```

## Context Manager

```python
with MemoryOS(base_url="http://localhost:8080") as client:
    client.add_memory("user_123", "user", "Hello")
```

## API Reference

### `MemoryOS(base_url, timeout=30)`

Initialize MemoryOS client.

**Parameters:**
- `base_url` (str): MemoryOS server URL
- `timeout` (int): Request timeout in seconds

### `add_memory(user_id, role, content, event_id=None)`

Add a memory entry.

**Parameters:**
- `user_id` (str): User ID
- `role` (str): Message role ("user" or "assistant")
- `content` (str): Message content
- `event_id` (str, optional): Event ID for deduplication

**Returns:** Dict with status

### `retrieve_context(user_id, query)`

Retrieve memory context.

**Parameters:**
- `user_id` (str): User ID
- `query` (str): Query text

**Returns:** Dict with short_term, mid_term, long_term memories

### `get_history(memory_id)`

Get memory history.

**Parameters:**
- `memory_id` (str): Memory ID

**Returns:** List of history entries

### `chat(messages, model="gpt-4o", stream=False, **kwargs)`

Chat with LLM.

**Parameters:**
- `messages` (list): List of message dicts
- `model` (str): Model name
- `stream` (bool): Enable streaming
- `**kwargs`: Additional parameters

**Returns:** Chat response dict

### `health_check()`

Check server health.

**Returns:** Health status dict

## License

Apache 2.0
