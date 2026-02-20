"""
MemoryOS Async Python SDK
Async/await support using aiohttp
"""

import json
import asyncio
import logging
from typing import List, Dict, Optional, Any, AsyncIterator

logger = logging.getLogger("memoryos.async")

try:
    import aiohttp

    AIOHTTP_AVAILABLE = True
except ImportError:
    AIOHTTP_AVAILABLE = False


class AsyncMemoryOSError(Exception):
    def __init__(
        self,
        message: str,
        status_code: Optional[int] = None,
    ):
        super().__init__(message)
        self.status_code = status_code


class AsyncConnectionError(AsyncMemoryOSError):
    pass


class AsyncAuthenticationError(AsyncMemoryOSError):
    pass


class AsyncRateLimitError(AsyncMemoryOSError):
    def __init__(
        self, message: str, retry_after: Optional[float] = None, **kwargs
    ):
        super().__init__(message, **kwargs)
        self.retry_after = retry_after


class AsyncMemoryOS:
    """Async MemoryOS Client using aiohttp."""

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        timeout: int = 30,
        max_retries: int = 3,
        retry_delay: float = 1.0,
    ):
        if not AIOHTTP_AVAILABLE:
            raise ImportError(
                "aiohttp is required for async support. Install with: pip install aiohttp"
            )

        self.base_url = base_url.rstrip("/")
        self.timeout = aiohttp.ClientTimeout(total=timeout)
        self.max_retries = max_retries
        self.retry_delay = retry_delay
        self._session: Optional[aiohttp.ClientSession] = None
        self._headers: Dict[str, str] = {"Content-Type": "application/json"}
        if api_key:
            self._headers["Authorization"] = f"Bearer {api_key}"

    async def _get_session(self) -> aiohttp.ClientSession:
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession(
                timeout=self.timeout, headers=self._headers
            )
        return self._session

    async def _request(
        self,
        method: str,
        path: str,
        json_data: Optional[Dict] = None,
    ) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        session = await self._get_session()
        last_exc: Optional[Exception] = None

        for attempt in range(self.max_retries):
            try:
                async with session.request(method, url, json=json_data) as resp:
                    if resp.status in (401, 403):
                        raise AsyncAuthenticationError(
                            f"Authentication failed: {resp.status}",
                            status_code=resp.status,
                        )
                    if resp.status == 429:
                        retry_after = float(
                            resp.headers.get(
                                "Retry-After", self.retry_delay * (attempt + 1)
                            )
                        )
                        if attempt < self.max_retries - 1:
                            logger.warning(
                                "Rate limited, retrying after %.1fs (attempt %d/%d)",
                                retry_after,
                                attempt + 1,
                                self.max_retries,
                            )
                            await asyncio.sleep(retry_after)
                            continue
                        raise AsyncRateLimitError(
                            "Rate limit exceeded",
                            retry_after=retry_after,
                            status_code=429,
                        )
                    if resp.status >= 500 and attempt < self.max_retries - 1:
                        delay = self.retry_delay * (2**attempt)
                        logger.warning(
                            "Server error %d, retrying after %.1fs (attempt %d/%d)",
                            resp.status,
                            delay,
                            attempt + 1,
                            self.max_retries,
                        )
                        await asyncio.sleep(delay)
                        continue
                    resp.raise_for_status()
                    return await resp.json()
            except aiohttp.ClientConnectionError as e:
                last_exc = e
                if attempt < self.max_retries - 1:
                    delay = self.retry_delay * (2**attempt)
                    logger.warning(
                        "Connection failed, retrying after %.1fs (attempt %d/%d)",
                        delay,
                        attempt + 1,
                        self.max_retries,
                    )
                    await asyncio.sleep(delay)
                    continue
                raise AsyncConnectionError(
                    f"Failed to connect to {url}: {e}"
                ) from e
            except asyncio.TimeoutError as e:
                last_exc = e
                if attempt < self.max_retries - 1:
                    delay = self.retry_delay * (2**attempt)
                    await asyncio.sleep(delay)
                    continue
                raise AsyncConnectionError(f"Request timed out: {url}") from e
            except (AsyncAuthenticationError, AsyncRateLimitError):
                raise

        raise AsyncConnectionError(
            f"Failed after {self.max_retries} retries: {last_exc}"
        )

    async def add_memory(
        self,
        user_id: str,
        role: str,
        content: str,
        event_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "user_id": user_id,
            "role": role,
            "content": content,
        }
        if event_id:
            payload["event_id"] = event_id
        return await self._request("POST", "/v1/memory/add", json_data=payload)

    async def retrieve_context(
        self, user_id: str, query: str
    ) -> Dict[str, Any]:
        payload = {"user_id": user_id, "query": query}
        return await self._request(
            "POST", "/v1/memory/retrieve", json_data=payload
        )

    async def get_history(self, memory_id: str) -> List[Dict[str, Any]]:
        return await self._request("GET", f"/v1/memory/{memory_id}/history")

    async def chat(
        self,
        messages: List[Dict[str, str]],
        model: str = "gpt-4o",
        stream: bool = False,
        **kwargs,
    ) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": stream,
            **kwargs,
        }
        return await self._request(
            "POST", "/v1/chat/completions", json_data=payload
        )

    async def chat_stream(
        self,
        messages: List[Dict[str, str]],
        model: str = "gpt-4o",
        **kwargs,
    ) -> AsyncIterator[Dict[str, Any]]:
        """Stream chat completions using SSE."""
        url = f"{self.base_url}/v1/chat/completions"
        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": True,
            **kwargs,
        }
        session = await self._get_session()
        async with session.post(url, json=payload) as resp:
            resp.raise_for_status()
            async for line in resp.content:
                line_str = line.decode("utf-8").strip()
                if not line_str:
                    continue
                if line_str.startswith("data: "):
                    data = line_str[6:]
                    if data.strip() == "[DONE]":
                        break
                    try:
                        yield json.loads(data)
                    except json.JSONDecodeError:
                        logger.warning("Failed to parse SSE chunk: %s", data)

    async def graph_extract(self, text: str) -> Dict[str, Any]:
        return await self._request(
            "POST", "/v1/graph/extract", json_data={"text": text}
        )

    async def graph_query(self, query: str) -> Dict[str, Any]:
        return await self._request(
            "POST", "/v1/graph/query", json_data={"query": query}
        )

    async def memory_export(
        self, user_id: str, fmt: str = "json"
    ) -> Dict[str, Any]:
        return await self._request(
            "POST",
            "/v1/memory/manage/export",
            json_data={"user_id": user_id, "format": fmt},
        )

    async def memory_import(
        self, user_id: str, segments: List[Dict]
    ) -> Dict[str, Any]:
        return await self._request(
            "POST",
            "/v1/memory/manage/import",
            json_data={"user_id": user_id, "segments": segments},
        )

    async def add_tags(
        self, user_id: str, segment_id: str, tags: List[str]
    ) -> Dict[str, Any]:
        return await self._request(
            "POST",
            "/v1/memory/manage/tags",
            json_data={
                "user_id": user_id,
                "segment_id": segment_id,
                "tags": tags,
            },
        )

    async def health_check(self) -> Dict[str, Any]:
        return await self._request("GET", "/health/status")

    async def close(self):
        if self._session and not self._session.closed:
            await self._session.close()

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()
