"""
MemoryOS Python SDK
High-performance AI Agent memory management system
"""

import json
import time
import logging
import requests
from typing import List, Dict, Optional, Any, Iterator

__version__ = "0.2.0"

logger = logging.getLogger("memoryos")


class MemoryOSError(Exception):
    """Base exception for MemoryOS SDK errors."""

    def __init__(self, message: str, status_code: Optional[int] = None, response: Optional[requests.Response] = None):
        super().__init__(message)
        self.status_code = status_code
        self.response = response


class ConnectionError(MemoryOSError):
    """Raised when connection to MemoryOS server fails."""


class AuthenticationError(MemoryOSError):
    """Raised on 401/403 responses."""


class RateLimitError(MemoryOSError):
    """Raised on 429 responses."""

    def __init__(self, message: str, retry_after: Optional[float] = None, **kwargs):
        super().__init__(message, **kwargs)
        self.retry_after = retry_after


class MemoryOS:
    """MemoryOS Client with retry support and streaming."""

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        timeout: int = 30,
        max_retries: int = 3,
        retry_delay: float = 1.0,
    ):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.max_retries = max_retries
        self.retry_delay = retry_delay
        self.session = requests.Session()
        if api_key:
            self.session.headers["Authorization"] = f"Bearer {api_key}"
        self.session.headers["Content-Type"] = "application/json"

    def _request(
        self,
        method: str,
        path: str,
        json_data: Optional[Dict] = None,
        stream: bool = False,
    ) -> requests.Response:
        url = f"{self.base_url}{path}"
        last_exc: Optional[Exception] = None

        for attempt in range(self.max_retries):
            try:
                response = self.session.request(
                    method,
                    url,
                    json=json_data,
                    timeout=self.timeout,
                    stream=stream,
                )
                if response.status_code == 401 or response.status_code == 403:
                    raise AuthenticationError(
                        f"Authentication failed: {response.status_code}",
                        status_code=response.status_code,
                        response=response,
                    )
                if response.status_code == 429:
                    retry_after = float(response.headers.get("Retry-After", self.retry_delay * (attempt + 1)))
                    if attempt < self.max_retries - 1:
                        logger.warning("Rate limited, retrying after %.1fs (attempt %d/%d)", retry_after, attempt + 1, self.max_retries)
                        time.sleep(retry_after)
                        continue
                    raise RateLimitError(
                        "Rate limit exceeded",
                        retry_after=retry_after,
                        status_code=429,
                        response=response,
                    )
                if response.status_code >= 500 and attempt < self.max_retries - 1:
                    delay = self.retry_delay * (2 ** attempt)
                    logger.warning("Server error %d, retrying after %.1fs (attempt %d/%d)", response.status_code, delay, attempt + 1, self.max_retries)
                    time.sleep(delay)
                    continue
                response.raise_for_status()
                return response
            except requests.exceptions.ConnectionError as e:
                last_exc = e
                if attempt < self.max_retries - 1:
                    delay = self.retry_delay * (2 ** attempt)
                    logger.warning("Connection failed, retrying after %.1fs (attempt %d/%d)", delay, attempt + 1, self.max_retries)
                    time.sleep(delay)
                    continue
                raise ConnectionError(f"Failed to connect to {url}: {e}") from e
            except requests.exceptions.Timeout as e:
                last_exc = e
                if attempt < self.max_retries - 1:
                    delay = self.retry_delay * (2 ** attempt)
                    logger.warning("Request timed out, retrying after %.1fs (attempt %d/%d)", delay, attempt + 1, self.max_retries)
                    time.sleep(delay)
                    continue
                raise ConnectionError(f"Request timed out: {url}") from e
            except (AuthenticationError, RateLimitError):
                raise
            except requests.exceptions.HTTPError as e:
                raise MemoryOSError(
                    f"HTTP error: {e}",
                    status_code=e.response.status_code if e.response is not None else None,
                    response=e.response,
                ) from e

        raise ConnectionError(f"Failed after {self.max_retries} retries: {last_exc}")

    def add_memory(self, user_id: str, role: str, content: str, event_id: Optional[str] = None) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "user_id": user_id,
            "role": role,
            "content": content,
        }
        if event_id:
            payload["event_id"] = event_id
        response = self._request("POST", "/v1/memory/add", json_data=payload)
        return response.json()

    def retrieve_context(self, user_id: str, query: str) -> Dict[str, Any]:
        payload = {"user_id": user_id, "query": query}
        response = self._request("POST", "/v1/memory/retrieve", json_data=payload)
        return response.json()

    def get_history(self, memory_id: str) -> List[Dict[str, Any]]:
        response = self._request("GET", f"/v1/memory/{memory_id}/history")
        return response.json()

    def chat(self, messages: List[Dict[str, str]], model: str = "gpt-4o", stream: bool = False, **kwargs) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": stream,
            **kwargs,
        }
        response = self._request("POST", "/v1/chat/completions", json_data=payload)
        return response.json()

    def chat_stream(self, messages: List[Dict[str, str]], model: str = "gpt-4o", **kwargs) -> Iterator[Dict[str, Any]]:
        """Stream chat completions using SSE."""
        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": True,
            **kwargs,
        }
        response = self._request("POST", "/v1/chat/completions", json_data=payload, stream=True)
        for line in response.iter_lines(decode_unicode=True):
            if not line:
                continue
            if line.startswith("data: "):
                data = line[6:]
                if data.strip() == "[DONE]":
                    break
                try:
                    yield json.loads(data)
                except json.JSONDecodeError:
                    logger.warning("Failed to parse SSE chunk: %s", data)

    def health_check(self) -> Dict[str, Any]:
        response = self._request("GET", "/health/status")
        return response.json()

    def close(self):
        self.session.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
