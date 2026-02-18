"""
MemoryOS Python SDK
High-performance AI Agent memory management system
"""

import requests
from typing import List, Dict, Optional, Any

__version__ = "0.2.0"

class MemoryOS:
    """MemoryOS Client"""
    
    def __init__(self, base_url: str = "http://localhost:8080", timeout: int = 30):
        self.base_url = base_url.rstrip('/')
        self.timeout = timeout
        self.session = requests.Session()
    
    def add_memory(self, user_id: str, role: str, content: str, event_id: Optional[str] = None) -> Dict[str, Any]:
        """添加记忆"""
        url = f"{self.base_url}/v1/memory/add"
        payload = {
            "user_id": user_id,
            "role": role,
            "content": content
        }
        if event_id:
            payload["event_id"] = event_id
        
        response = self.session.post(url, json=payload, timeout=self.timeout)
        response.raise_for_status()
        return response.json()
    
    def retrieve_context(self, user_id: str, query: str) -> Dict[str, Any]:
        """检索上下文"""
        url = f"{self.base_url}/v1/memory/retrieve"
        payload = {
            "user_id": user_id,
            "query": query
        }
        
        response = self.session.post(url, json=payload, timeout=self.timeout)
        response.raise_for_status()
        return response.json()
    
    def get_history(self, memory_id: str) -> List[Dict[str, Any]]:
        """获取记忆历史"""
        url = f"{self.base_url}/v1/memory/{memory_id}/history"
        
        response = self.session.get(url, timeout=self.timeout)
        response.raise_for_status()
        return response.json()
    
    def chat(self, messages: List[Dict[str, str]], model: str = "gpt-4o", stream: bool = False, **kwargs) -> Dict[str, Any]:
        """聊天"""
        url = f"{self.base_url}/v1/chat/completions"
        payload = {
            "model": model,
            "messages": messages,
            "stream": stream,
            **kwargs
        }
        
        response = self.session.post(url, json=payload, timeout=self.timeout)
        response.raise_for_status()
        return response.json()
    
    def health_check(self) -> Dict[str, Any]:
        """健康检查"""
        url = f"{self.base_url}/health/status"
        response = self.session.get(url, timeout=self.timeout)
        response.raise_for_status()
        return response.json()
    
    def close(self):
        """关闭连接"""
        self.session.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
