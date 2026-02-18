# Mem0 源码分析与实现计划

**日期**: 2026-02-17  
**分析对象**: Mem0 开源代码  
**目标**: 实现知识图谱、记忆版本控制、多语言 SDK

---

## 📊 Mem0 源码分析

### 1. 知识图谱 (Knowledge Graph)

#### 核心文件
- `mem0/graphs/tools.py` - 图操作工具定义
- `mem0/graphs/configs.py` - 图数据库配置
- `mem0/graphs/neptune/` - AWS Neptune 实现
- `mem0/graphs/utils.py` - 图工具函数

#### 核心概念

**节点 (Node)**:
```python
{
    "name": "Alice",
    "type": "Person"  # 节点类型
}
```

**关系 (Relationship)**:
```python
{
    "source": "Alice",
    "source_type": "Person",
    "destination": "Google",
    "destination_type": "Company",
    "relationship": "works_at"
}
```

#### LLM 工具调用

Mem0 使用 **Function Calling** 让 LLM 提取实体和关系：

```python
# 1. ADD_MEMORY_TOOL_GRAPH - 添加新关系
{
    "name": "add_graph_memory",
    "parameters": {
        "source": "Alice",
        "destination": "Google",
        "relationship": "works_at",
        "source_type": "Person",
        "destination_type": "Company"
    }
}

# 2. UPDATE_MEMORY_TOOL_GRAPH - 更新关系
{
    "name": "update_graph_memory",
    "parameters": {
        "source": "Alice",
        "destination": "Google",
        "relationship": "worked_at"  # 更新关系
    }
}

# 3. NOOP_TOOL - 无操作
{
    "name": "noop"
}
```

#### 存储后端

**支持的图数据库**:
- AWS Neptune (NeptuneDB / NeptuneGraph)
- Memgraph (未在开源代码中)
- Mermaid (可视化)

#### 工作流程

```
User Input: "Alice works at Google as a software engineer"
    ↓
LLM + Function Calling
    ↓
Extract Entities & Relationships:
  - Entity 1: Alice (Person)
  - Entity 2: Google (Company)
  - Relationship: works_at
    ↓
Store in Graph DB:
  (Alice:Person) -[works_at]-> (Google:Company)
    ↓
Query: "Where does Alice work?"
    ↓
Graph Query:
  MATCH (p:Person {name: "Alice"})-[r:works_at]->(c:Company)
  RETURN c.name
    ↓
Result: "Google"
```

---

### 2. 记忆版本控制 (Memory History)

#### 核心文件
- `mem0/memory/storage.py` - SQLite 历史存储
- `mem0/memory/main.py` - 历史记录逻辑

#### 数据库表结构

```sql
CREATE TABLE history (
    id           TEXT PRIMARY KEY,
    memory_id    TEXT,           -- 记忆 ID
    old_memory   TEXT,           -- 旧内容
    new_memory   TEXT,           -- 新内容
    event        TEXT,           -- 事件类型: ADD/UPDATE/DELETE
    created_at   DATETIME,
    updated_at   DATETIME,
    is_deleted   INTEGER,
    actor_id     TEXT,           -- 操作者 ID
    role         TEXT            -- 操作者角色
)
```

#### 事件类型

1. **ADD** - 添加新记忆
   ```python
   {
       "memory_id": "uuid",
       "old_memory": None,
       "new_memory": "Alice likes pizza",
       "event": "ADD"
   }
   ```

2. **UPDATE** - 更新记忆
   ```python
   {
       "memory_id": "uuid",
       "old_memory": "Alice likes pizza",
       "new_memory": "Alice loves pizza and pasta",
       "event": "UPDATE"
   }
   ```

3. **DELETE** - 删除记忆
   ```python
   {
       "memory_id": "uuid",
       "old_memory": "Alice loves pizza and pasta",
       "new_memory": None,
       "event": "DELETE",
       "is_deleted": 1
   }
   ```

#### API 接口

```python
# 获取记忆历史
def history(self, memory_id):
    """
    Get the history of changes for a memory by ID.
    
    Returns:
        list: [
            {
                "id": "history_uuid",
                "memory_id": "memory_uuid",
                "old_memory": "...",
                "new_memory": "...",
                "event": "UPDATE",
                "created_at": "2026-02-17T10:00:00",
                "actor_id": "user_123"
            },
            ...
        ]
    """
    return self.db.get_history(memory_id)
```

#### 工作流程

```
1. 添加记忆
   User: "Alice likes pizza"
   → Vector Store: 存储向量
   → History DB: 记录 ADD 事件

2. 更新记忆
   User: "Alice loves pizza and pasta"
   → Vector Store: 更新向量
   → History DB: 记录 UPDATE 事件 (保留旧内容)

3. 查询历史
   GET /v1/memories/{memory_id}/history
   → History DB: 查询所有变更
   → 返回时间线
```

---

### 3. Python SDK

#### 核心文件
- `mem0/client/main.py` - 同步客户端
- `mem0/client/project.py` - 项目管理

#### SDK 架构

```python
class MemoryClient:
    def __init__(self, api_key, host="https://api.mem0.ai"):
        self.client = httpx.Client(
            base_url=host,
            headers={"Authorization": f"Token {api_key}"}
        )
    
    def add(self, messages, **kwargs):
        """添加记忆"""
        return self.client.post("/v1/memories/", json={...})
    
    def get(self, memory_id):
        """获取记忆"""
        return self.client.get(f"/v1/memories/{memory_id}")
    
    def search(self, query, **kwargs):
        """搜索记忆"""
        return self.client.post("/v1/memories/search/", json={...})
    
    def update(self, memory_id, data):
        """更新记忆"""
        return self.client.put(f"/v1/memories/{memory_id}", json=data)
    
    def delete(self, memory_id):
        """删除记忆"""
        return self.client.delete(f"/v1/memories/{memory_id}")
    
    def history(self, memory_id):
        """获取历史"""
        return self.client.get(f"/v1/memories/{memory_id}/history")
```

#### 异步客户端

```python
class AsyncMemoryClient:
    def __init__(self, api_key, host="https://api.mem0.ai"):
        self.client = httpx.AsyncClient(...)
    
    async def add(self, messages, **kwargs):
        return await self.client.post(...)
    
    # ... 其他异步方法
```

---

## 🎯 MemoryOS-Rust 实现计划

### Phase 1: 记忆版本控制 (1-2 天)

#### 1.1 数据库设计

```sql
-- PostgreSQL / SQLite
CREATE TABLE memory_history (
    id           UUID PRIMARY KEY,
    memory_id    UUID NOT NULL,
    old_content  TEXT,
    new_content  TEXT,
    event_type   VARCHAR(20) NOT NULL,  -- ADD/UPDATE/DELETE
    created_at   TIMESTAMP NOT NULL,
    updated_at   TIMESTAMP NOT NULL,
    is_deleted   BOOLEAN DEFAULT FALSE,
    actor_id     VARCHAR(255),
    actor_role   VARCHAR(50),
    metadata     JSONB
);

CREATE INDEX idx_memory_history_memory_id ON memory_history(memory_id);
CREATE INDEX idx_memory_history_created_at ON memory_history(created_at DESC);
```

#### 1.2 Rust 实现

```rust
// crates/memoryos-core/src/history.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHistoryEntry {
    pub id: String,
    pub memory_id: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub event_type: HistoryEventType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
    pub actor_id: Option<String>,
    pub actor_role: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryEventType {
    Add,
    Update,
    Delete,
}

// crates/memoryos-ports/src/history.rs
#[async_trait]
pub trait HistoryStorage: Send + Sync {
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError>;
    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError>;
    async fn get_entry(&self, id: &str) -> Result<Option<MemoryHistoryEntry>, AppError>;
}

// crates/memoryos-adapters/src/history/postgres.rs
pub struct PostgresHistoryStorage {
    pool: PgPool,
}

impl PostgresHistoryStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HistoryStorage for PostgresHistoryStorage {
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO memory_history 
            (id, memory_id, old_content, new_content, event_type, created_at, updated_at, is_deleted, actor_id, actor_role, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            entry.id,
            entry.memory_id,
            entry.old_content,
            entry.new_content,
            entry.event_type.to_string(),
            entry.created_at,
            entry.updated_at,
            entry.is_deleted,
            entry.actor_id,
            entry.actor_role,
            entry.metadata
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError> {
        let rows = sqlx::query_as!(
            MemoryHistoryEntry,
            r#"
            SELECT * FROM memory_history 
            WHERE memory_id = $1 
            ORDER BY created_at DESC
            "#,
            memory_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
```

#### 1.3 API 端点

```rust
// GET /v1/memory/{memory_id}/history
async fn get_memory_history(
    Path(memory_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MemoryHistoryEntry>>, AppError> {
    let history = state.history_storage.get_history(&memory_id).await?;
    Ok(Json(history))
}
```

---

### Phase 2: 知识图谱 (3-5 天)

#### 2.1 选择图数据库

**推荐**: Qdrant + Mermaid (无需额外部署)

**备选**:
- Memgraph (高性能，兼容 Cypher)
- SurrealDB (Rust 原生，多模型)

#### 2.2 数据模型

```rust
// crates/memoryos-core/src/graph.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub name: String,
    pub node_type: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    pub source: String,
    pub source_type: String,
    pub destination: String,
    pub destination_type: String,
    pub relationship: String,
    pub properties: HashMap<String, serde_json::Value>,
}

// crates/memoryos-ports/src/graph.rs
#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn add_node(&self, node: GraphNode) -> Result<(), AppError>;
    async fn add_relationship(&self, rel: GraphRelationship) -> Result<(), AppError>;
    async fn update_relationship(&self, source: &str, dest: &str, new_rel: &str) -> Result<(), AppError>;
    async fn query(&self, cypher: &str) -> Result<Vec<serde_json::Value>, AppError>;
    async fn get_neighbors(&self, node_name: &str) -> Result<Vec<GraphNode>, AppError>;
}
```

#### 2.3 LLM 实体提取

```rust
// crates/memoryos-core/src/graph/extraction.rs
pub struct EntityExtractor {
    llm: Arc<dyn LlmAdapter>,
}

impl EntityExtractor {
    pub async fn extract(&self, text: &str) -> Result<Vec<GraphRelationship>, AppError> {
        let tools = vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "add_graph_memory",
                    "description": "Add a new relationship to the knowledge graph",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "source": {"type": "string"},
                            "destination": {"type": "string"},
                            "relationship": {"type": "string"},
                            "source_type": {"type": "string"},
                            "destination_type": {"type": "string"}
                        },
                        "required": ["source", "destination", "relationship", "source_type", "destination_type"]
                    }
                }
            })
        ];
        
        let request = ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "Extract entities and relationships from the text.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                }
            ],
            tools: Some(tools),
            tool_choice: Some("auto".to_string()),
            ..Default::default()
        };
        
        let response = self.llm.chat(request).await?;
        
        // 解析 tool_calls
        let relationships = parse_tool_calls(response)?;
        Ok(relationships)
    }
}
```

#### 2.4 Mermaid 图谱生成

```rust
// crates/memoryos-wiki/src/mermaid.rs
use memoryos_ports::VectorStorage;

pub struct MermaidGenerator {
    qdrant: Arc<dyn VectorStorage>,
}

impl MermaidGenerator {
    pub async fn new(qdrant: Arc<dyn VectorStorage>) -> Result<Self, AppError> {
        Ok(Self { qdrant })
    }
    
    pub async fn generate_graph(&self, user_id: &str) -> Result<String, AppError> {
        // 从 Qdrant 查询关系
        let relations = self.query_relations(user_id).await?;
        
        // 生成 Mermaid 语法
        let mut mermaid = String::from("graph TD\n");
        for rel in relations {
            mermaid.push_str(&format!(
                "    {}[{}] -->|{}| {}[{}]\n",
                rel.from_id, rel.from_label,
                rel.relation_type,
                rel.to_id, rel.to_label
            ));
        }
        
        Ok(mermaid))
    }
    
    async fn add_relationship(&self, rel: GraphRelationship) -> Result<(), AppError> {
        // 1. 创建源节点
        self.add_node(GraphNode {
            name: rel.source.clone(),
            node_type: rel.source_type.clone(),
            properties: HashMap::new(),
        }).await?;
        
        // 2. 创建目标节点
        self.add_node(GraphNode {
            name: rel.destination.clone(),
            node_type: rel.destination_type.clone(),
            properties: HashMap::new(),
        }).await?;
        
        // 3. 创建关系
        let q = query(&format!(
            "MATCH (a:{} {{name: $source}}), (b:{} {{name: $dest}}) 
             MERGE (a)-[r:{}]->(b) 
             SET r += $props",
            rel.source_type, rel.destination_type, rel.relationship
        ))
        .param("source", rel.source)
        .param("dest", rel.destination)
        .param("props", rel.properties);
        
        self.graph.run(q).await?;
        Ok(())
    }
    
    async fn query(&self, cypher: &str) -> Result<Vec<serde_json::Value>, AppError> {
        let mut result = self.graph.execute(query(cypher)).await?;
        let mut rows = Vec::new();
        
        while let Some(row) = result.next().await? {
            rows.push(row.to::<serde_json::Value>()?);
        }
        
        Ok(rows)
    }
}
```

---

### Phase 3: 多语言 SDK (2-3 天)

#### 3.1 SDK 架构

**统一 HTTP API** → **多语言客户端**

```
MemoryOS-Rust HTTP API
    ↓
┌───────────┬───────────┬───────────┬───────────┬───────────┐
│  Python   │   Java    │    Go     │   Rust    │  Node.js  │
│    SDK    │    SDK    │    SDK    │    SDK    │    SDK    │
└───────────┴───────────┴───────────┴───────────┴───────────┘
```

#### 3.2 Python SDK

```python
# memoryos-sdk-python/memoryos/client.py
import httpx
from typing import List, Dict, Any, Optional

class MemoryOSClient:
    def __init__(self, base_url: str = "http://localhost:8080", api_key: Optional[str] = None):
        self.base_url = base_url
        self.client = httpx.Client(
            base_url=base_url,
            headers={"Authorization": f"Bearer {api_key}"} if api_key else {}
        )
    
    def add_memory(self, user_id: str, message: Dict[str, str]) -> Dict[str, Any]:
        """添加记忆"""
        return self.client.post(
            "/v1/memory/add",
            json={"user_id": user_id, "message": message}
        ).json()
    
    def retrieve_memory(self, user_id: str, query: str, limit: int = 5) -> List[Dict[str, Any]]:
        """检索记忆"""
        return self.client.post(
            "/v1/memory/retrieve",
            json={"user_id": user_id, "query": query, "limit": limit}
        ).json()
    
    def get_profile(self, user_id: str) -> Dict[str, Any]:
        """获取用户画像"""
        return self.client.get(f"/v1/memory/profile/{user_id}").json()
    
    def get_history(self, memory_id: str) -> List[Dict[str, Any]]:
        """获取记忆历史"""
        return self.client.get(f"/v1/memory/{memory_id}/history").json()
    
    def add_graph_memory(self, user_id: str, relationship: Dict[str, str]) -> Dict[str, Any]:
        """添加图记忆"""
        return self.client.post(
            "/v1/graph/add",
            json={"user_id": user_id, **relationship}
        ).json()
    
    def query_graph(self, user_id: str, query: str) -> List[Dict[str, Any]]:
        """查询知识图谱"""
        return self.client.post(
            "/v1/graph/query",
            json={"user_id": user_id, "query": query}
        ).json()

# 异步版本
class AsyncMemoryOSClient:
    def __init__(self, base_url: str = "http://localhost:8080", api_key: Optional[str] = None):
        self.base_url = base_url
        self.client = httpx.AsyncClient(...)
    
    async def add_memory(self, user_id: str, message: Dict[str, str]) -> Dict[str, Any]:
        response = await self.client.post(...)
        return response.json()
```

#### 3.3 其他语言 SDK

**Java SDK**:
```java
// memoryos-sdk-java/src/main/java/com/memoryos/MemoryOSClient.java
public class MemoryOSClient {
    private final HttpClient client;
    private final String baseUrl;
    
    public MemoryOSClient(String baseUrl) {
        this.baseUrl = baseUrl;
        this.client = HttpClient.newHttpClient();
    }
    
    public CompletableFuture<JsonObject> addMemory(String userId, JsonObject message) {
        // HTTP POST /v1/memory/add
    }
    
    public CompletableFuture<List<JsonObject>> retrieveMemory(String userId, String query) {
        // HTTP POST /v1/memory/retrieve
    }
}
```

**Go SDK**:
```go
// memoryos-sdk-go/client.go
package memoryos

type Client struct {
    baseURL string
    client  *http.Client
}

func NewClient(baseURL string) *Client {
    return &Client{
        baseURL: baseURL,
        client:  &http.Client{},
    }
}

func (c *Client) AddMemory(ctx context.Context, userID string, message Message) (*Response, error) {
    // HTTP POST /v1/memory/add
}

func (c *Client) RetrieveMemory(ctx context.Context, userID string, query string) ([]Memory, error) {
    // HTTP POST /v1/memory/retrieve
}
```

**Rust SDK**:
```rust
// memoryos-sdk-rust/src/lib.rs
pub struct MemoryOSClient {
    base_url: String,
    client: reqwest::Client,
}

impl MemoryOSClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn add_memory(&self, user_id: &str, message: Message) -> Result<Response> {
        // HTTP POST /v1/memory/add
    }
    
    pub async fn retrieve_memory(&self, user_id: &str, query: &str) -> Result<Vec<Memory>> {
        // HTTP POST /v1/memory/retrieve
    }
}
```

**Node.js SDK**:
```typescript
// memoryos-sdk-nodejs/src/index.ts
export class MemoryOSClient {
    private baseURL: string;
    private client: AxiosInstance;
    
    constructor(baseURL: string = 'http://localhost:8080') {
        this.baseURL = baseURL;
        this.client = axios.create({ baseURL });
    }
    
    async addMemory(userId: string, message: Message): Promise<Response> {
        return this.client.post('/v1/memory/add', { user_id: userId, message });
    }
    
    async retrieveMemory(userId: string, query: string): Promise<Memory[]> {
        return this.client.post('/v1/memory/retrieve', { user_id: userId, query });
    }
}
```

---

## 📅 实施时间表

### Week 1: 记忆版本控制
- Day 1-2: 数据库设计 + Rust 实现
- Day 3: API 端点 + 测试
- Day 4: 文档

### Week 2: 知识图谱
- Day 1-2: Mermaid 图谱生成 + 基础可视化
- Day 3-4: LLM 实体提取
- Day 5: 图查询 API + 测试

### Week 3: 多语言 SDK
- Day 1: Python SDK
- Day 2: Java SDK + Go SDK
- Day 3: Rust SDK + Node.js SDK
- Day 4-5: 测试 + 文档 + 发布

---

## 🎯 总结

### Mem0 核心特性

1. **知识图谱**: LLM Function Calling + Qdrant + Mermaid
2. **记忆版本控制**: SQLite 历史表 + 事件追踪
3. **Python SDK**: httpx 客户端 + 同步/异步

### MemoryOS-Rust 实现策略

1. **记忆版本控制**: PostgreSQL + SQLx (最简单，1-2 天)
2. **知识图谱**: Qdrant + Mermaid + LLM Function Calling (简单，2-3 天)
3. **多语言 SDK**: 统一 HTTP API + 5 种语言客户端 (简单，2-3 天)

**总耗时**: 6-10 天

**优先级**: 记忆版本控制 > 知识图谱 > 多语言 SDK

---

**准备好开始实现了吗？我建议从记忆版本控制开始，因为它最简单且最实用！**
