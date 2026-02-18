"""
Example: Basic usage of MemoryOS Python SDK
"""

from memoryos import MemoryOS

def main():
    # Initialize client
    client = MemoryOS(base_url="http://localhost:8080")
    
    # Add memories
    print("Adding memories...")
    client.add_memory("alice", "user", "I love Python and Rust")
    client.add_memory("alice", "assistant", "That's great! Both are powerful languages.")
    client.add_memory("alice", "user", "I'm working on a memory system project")
    
    # Retrieve context
    print("\nRetrieving context...")
    context = client.retrieve_context("alice", "What am I working on?")
    print(f"Context: {context}")
    
    # Chat
    print("\nChatting...")
    response = client.chat(
        messages=[
            {"role": "user", "content": "What do you know about me?"}
        ],
        model="gpt-4o"
    )
    print(f"Response: {response}")
    
    # Health check
    print("\nHealth check...")
    status = client.health_check()
    print(f"Status: {status}")
    
    client.close()

if __name__ == "__main__":
    main()
