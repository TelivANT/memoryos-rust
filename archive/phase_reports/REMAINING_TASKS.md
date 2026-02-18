# 剩余 13% 功能实现清单

**目标**: 从 87% → 100% 功能完整度  
**预计时间**: 2-3 天  
**优先级**: 按影响力排序

---

## 🎯 必做任务 (7% → 94%)

### 1. 知识图谱 (7%) 🔴 高优先级

**为什么重要**: Mem0 的核心差异化功能，支持复杂关系推理

#### 1.1 知识图谱可视化 (2 小时)
```rust
// crates/memoryos-wiki/src/mermaid.rs
pub struct MermaidGenerator {
    qdrant: Arc<QdrantClient>,
}

impl MermaidGenerator {
    pub async fn generate_graph(&self, user_id: &str) -> Result<String>;
    async fn query_relations(&self, user_id: &str) -> Result<Vec<Relation>>;
    fn format_mermaid(&self, relations: &[Relation]) -> String;
}
```

**存储**: Qdrant payload 存储关系

#### 1.2 实体提取 (3 小时)
```rust
// crates/memoryos-core/src/graph/extractor.rs
pub struct EntityExtractor {
    llm: Arc<dyn LlmAdapter>,
}

impl EntityExtractor {
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<Entity>>;
    pub async fn extract_relations(&self, text: &str) -> Result<Vec<Relation>>;
}

pub struct Entity {
    pub name: String,
    pub entity_type: String, // Person, Place, Organization, Concept
    pub properties: HashMap<String, String>,
}

pub struct Relation {
    pub from: String,
    pub to: String,
    pub relation_type: String, // knows, works_at, likes, etc.
}
```

**实现方式**: LLM Function Calling (OpenAI/Gemini)

#### 1.3 图查询 API (1 小时)
```rust
// crates/memoryos-gateway/src/routes/graph.rs
POST /v1/graph/query
{
  "user_id": "user_123",
  "query": "Who does Alice know?",
  "cypher": "MATCH (a:Person {name: 'Alice'})-[:KNOWS]->(b) RETURN b"
}

GET /v1/graph/entities/{user_id}
GET /v1/graph/relations/{user_id}
```

#### 1.4 集成到 MemoryManager (1 小时)
```rust
// crates/memoryos-adapters/src/memory/manager.rs
pub struct DefaultMemoryManager {
    // ... existing fields
    graph_storage: Option<Arc<dyn GraphStorage>>,
}

impl DefaultMemoryManager {
    async fn add_message_with_graph(&self, user_id: &str, message: Message) -> Result<()> {
        // 1. 添加到记忆
        self.add_message(user_id, message.clone()).await?;
        
        // 2. 提取实体和关系
        if let Some(graph) = &self.graph_storage {
            let entities = self.entity_extractor.extract_entities(&message.content).await?;
            let relations = self.entity_extractor.extract_relations(&message.content).await?;
            
            // 3. 存储到图数据库
            for entity in entities {
                graph.add_entity(user_id, entity).await?;
            }
            for relation in relations {
                graph.add_relation(user_id, relation).await?;
            }
        }
        
        Ok(())
    }
}
```

**预计时间**: 7 小时  
**完成后**: 87% → 94%

---

## 🟡 可选任务 (6% → 100%)

### 2. Python SDK (3%) 🟡 中优先级

**为什么有用**: 方便 Python 开发者使用

#### 2.1 基础 SDK (2 小时)
```python
# memoryos-python/memoryos/__init__.py
class MemoryOS:
    def __init__(self, base_url: str, api_key: str = None):
        self.base_url = base_url
        self.api_key = api_key
    
    def add_memory(self, user_id: str, role: str, content: str) -> dict:
        """添加记忆"""
        
    def retrieve_context(self, user_id: str, query: str) -> dict:
        """检索上下文"""
    
    def get_history(self, memory_id: str) -> list:
        """获取历史"""
    
    def chat(self, messages: list, model: str = "gpt-4o") -> dict:
        """聊天"""
```

#### 2.2 发布到 PyPI (1 小时)
```bash
# setup.py
python setup.py sdist bdist_wheel
twine upload dist/*
```

**预计时间**: 3 小时  
**完成后**: 94% → 97%

---

### 3. 更多 LLM 提供商 (2%) 🟢 低优先级

**为什么可选**: 已有 7 个主流 LLM，基本够用

#### 3.1 Groq (30 分钟)
```rust
// crates/memoryos-adapters/src/llm/groq.rs
pub struct GroqAdapter {
    client: Client,
    api_key: String,
    base_url: String, // https://api.groq.com/openai/v1
}
```

#### 3.2 AWS Bedrock (1 小时)
```rust
// crates/memoryos-adapters/src/llm/bedrock.rs
pub struct BedrockAdapter {
    client: aws_sdk_bedrockruntime::Client,
    model_id: String,
}
```

#### 3.3 Cohere (30 分钟)
```rust
// crates/memoryos-adapters/src/llm/cohere.rs
pub struct CohereAdapter {
    client: Client,
    api_key: String,
}
```

**预计时间**: 2 小时  
**完成后**: 97% → 99%

---

### 4. 更多向量数据库 (1%) 🟢 低优先级

**为什么可选**: Qdrant 已足够强大

#### 4.1 Chroma (1 小时)
```rust
// crates/memoryos-adapters/src/memory/chroma.rs
pub struct ChromaStorage {
    client: chromadb::Client,
    collection: String,
}
```

**预计时间**: 1 小时  
**完成后**: 99% → 100%

---

## 📋 实现计划

### 阶段 1: 知识图谱 (必做) - 1 天
- [ ] Mermaid 图谱生成 (2h)
- [ ] 实体提取器 (3h)
- [ ] 图查询 API (1h)
- [ ] 集成到 MemoryManager (1h)
- [ ] 测试 (2h)

**完成后**: 87% → 94%

---

### 阶段 2: Python SDK (可选) - 0.5 天
- [ ] 基础 SDK (2h)
- [ ] 测试 (1h)
- [ ] 发布到 PyPI (1h)

**完成后**: 94% → 97%

---

### 阶段 3: 更多 LLM (可选) - 0.5 天
- [ ] Groq 适配器 (0.5h)
- [ ] AWS Bedrock 适配器 (1h)
- [ ] Cohere 适配器 (0.5h)
- [ ] 测试 (1h)

**完成后**: 97% → 99%

---

### 阶段 4: 更多向量库 (可选) - 0.5 天
- [ ] Chroma 适配器 (1h)
- [ ] 测试 (1h)

**完成后**: 99% → 100%

---

## 🎯 推荐方案

### 方案 A: 最小可行 (1 天)
✅ **只做知识图谱** → 94%

**理由**:
- 知识图谱是 Mem0 的核心差异
- 其他功能可选，不影响核心使用
- 快速达到功能对等

---

### 方案 B: 完整实现 (2.5 天)
✅ **知识图谱 + Python SDK + 更多 LLM** → 99%

**理由**:
- Python SDK 提升易用性
- 更多 LLM 提升灵活性
- 接近 100% 功能完整度

---

### 方案 C: 完美主义 (3 天)
✅ **全部实现** → 100%

**理由**:
- 100% 功能对等 Mem0
- 完整的生态系统
- 最大化竞争力

---

## 🚀 我的建议

**推荐方案 A**: 只做知识图谱 (1 天)

**理由**:
1. ✅ 知识图谱是最大差距（7%）
2. ✅ 其他功能可选，不影响核心
3. ✅ 快速达到 94%，接近 Mem0 的 88%
4. ✅ 后续可按需添加

**实现顺序**:
1. Mermaid 图谱生成 (2h)
2. 实体提取器 (3h)
3. 图查询 API (1h)
4. 集成测试 (2h)

**总计**: 8 小时（1 个工作日）

---

## 📝 任务分解

### 知识图谱实现 (详细步骤)

#### Step 1: 添加依赖 (5 分钟)
```toml
# Cargo.toml
[dependencies]
neo4rs = "0.7"
```

#### Step 2: Mermaid 生成器 (2 小时)
```bash
# 创建文件
crates/memoryos-wiki/src/mermaid.rs
crates/memoryos-wiki/src/graph.rs
```

#### Step 3: 实体提取器 (3 小时)
```bash
# 创建文件
crates/memoryos-core/src/graph/extractor.rs
crates/memoryos-core/src/graph/entity.rs
```

#### Step 4: API 路由 (1 小时)
```bash
# 创建文件
crates/memoryos-gateway/src/routes/graph.rs
```

#### Step 5: 集成测试 (2 小时)
```bash
# 创建文件
tests/graph_integration.rs
```

---

## ✅ 验收标准

### 知识图谱验收
- [ ] Mermaid 图谱生成成功
- [ ] 实体提取准确率 > 80%
- [ ] 关系提取准确率 > 70%
- [ ] 图查询 API 正常工作
- [ ] 集成测试通过
- [ ] 文档完整

---

**你的选择**: 方案 A / B / C？

我建议先做**方案 A（知识图谱）**，1 天完成，达到 94%。其他功能可以后续按需添加。
