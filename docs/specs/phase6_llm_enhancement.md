# Phase 6 技术方案 - LLM 功能完善

**版本**: v1.0  
**创建时间**: 2026-02-17

---

## 1. 真实 LLM 总结

### 1.1 需求分析

**当前问题**:
```rust
// 直接拼接对话，没有真正总结
let summary = messages.iter()
    .map(|m| format!("{}: {}", m.role, m.content))
    .join("\n");
```

**目标**:
- 使用 LLM 生成高质量摘要
- 保留关键信息（人名、时间、事件）
- 摘要长度 < 原文 50%

### 1.2 技术方案

#### Trait 扩展
```rust
// crates/memoryos-ports/src/llm.rs
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    // 现有方法
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError>;
    
    // 新增：总结方法
    async fn summarize(
        &self,
        messages: &[Message],
        max_length: Option<usize>,
    ) -> Result<String, AppError> {
        // 默认实现
        let conversation = messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        
        let system_prompt = "你是一个对话总结助手。请提取以下对话的核心内容，保留关键信息（人名、时间、地点、事件），生成简洁的摘要。";
        
        let user_prompt = format!(
            "请总结以下对话（目标长度：{}字以内）：\n\n{}",
            max_length.unwrap_or(500),
            conversation
        );
        
        let request = ChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(1000),
            stream: false,
        };
        
        let response = self.chat(request).await?;
        Ok(response.choices[0].message.content.clone())
    }
}
```

#### 使用示例
```rust
// crates/memoryos-adapters/src/memory/manager.rs
async fn summarize_messages_internal(&self, messages: &[Message]) -> Result<String, AppError> {
    // 调用 LLM 生成摘要
    let summary = self.llm.summarize(messages, Some(500)).await?;
    
    Ok(format!(
        "[Consolidated at {}] {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
        summary
    ))
}
```

### 1.3 质量评估

#### 评估指标
```rust
struct SummaryQuality {
    compression_ratio: f32,  // 压缩比（摘要长度/原文长度）
    key_info_preserved: bool, // 关键信息是否保留
    coherence_score: f32,     // 连贯性评分
}

async fn evaluate_summary(
    original: &[Message],
    summary: &str,
) -> Result<SummaryQuality, AppError> {
    let original_len = original.iter()
        .map(|m| m.content.len())
        .sum::<usize>();
    
    let summary_len = summary.len();
    let compression_ratio = summary_len as f32 / original_len as f32;
    
    // 检查关键信息（简化版）
    let key_info_preserved = check_key_info(original, summary);
    
    Ok(SummaryQuality {
        compression_ratio,
        key_info_preserved,
        coherence_score: 0.8, // TODO: 使用 LLM 评估
    })
}
```

### 1.4 配置选项
```toml
[memory.consolidation]
summarize_enabled = true
summarize_model = "gpt-4o-mini"
max_summary_length = 500
temperature = 0.3
```

---

## 2. 真实 Profile 提取

### 2.1 需求分析

**当前问题**:
```rust
// 简单规则匹配
if text.contains("i like") {
    preferences.push(extract_after("i like"));
}
```

**目标**:
- 使用 LLM 结构化提取
- 支持增量更新
- 输出 JSON 格式

### 2.2 技术方案

#### Trait 扩展
```rust
// crates/memoryos-ports/src/llm.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedProfile {
    pub traits: Vec<String>,
    pub preferences: Vec<String>,
    pub background: Option<String>,
    pub knowledge: Vec<String>,
    pub confidence: f32,
}

#[async_trait]
pub trait LlmAdapter: Send + Sync {
    // 新增：Profile 提取
    async fn extract_profile(
        &self,
        messages: &[Message],
    ) -> Result<ExtractedProfile, AppError> {
        let conversation = messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        
        let system_prompt = r#"你是一个用户画像提取助手。请从对话中提取用户的特征、偏好、背景和知识领域。

输出 JSON 格式：
{
  "traits": ["性格特征1", "性格特征2"],
  "preferences": ["偏好1", "偏好2"],
  "background": "背景描述",
  "knowledge": ["知识领域1", "知识领域2"],
  "confidence": 0.85
}

注意：
- traits: 性格特征（如：友好、专业、幽默）
- preferences: 偏好（如：喜欢 Rust、喜欢开源）
- background: 背景（如：5年经验的软件工程师）
- knowledge: 知识领域（如：分布式系统、AI/ML）
- confidence: 置信度（0-1）
"#;
        
        let user_prompt = format!("请提取以下对话中的用户画像：\n\n{}", conversation);
        
        let request = ChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(1000),
            stream: false,
        };
        
        let response = self.chat(request).await?;
        let content = &response.choices[0].message.content;
        
        // 解析 JSON
        let profile: ExtractedProfile = serde_json::from_str(content)
            .map_err(|e| AppError::Internal(format!("Failed to parse profile: {}", e)))?;
        
        Ok(profile)
    }
}
```

#### 增量更新
```rust
// crates/memoryos-adapters/src/memory/manager.rs
async fn merge_profiles(
    &self,
    old_profile: &UserProfile,
    new_profile: &ExtractedProfile,
) -> UserProfile {
    let mut merged = old_profile.clone();
    
    // 合并 traits（去重）
    for trait_item in &new_profile.traits {
        if !merged.traits.contains(trait_item) {
            merged.traits.push(trait_item.clone());
        }
    }
    
    // 合并 preferences（去重）
    for pref in &new_profile.preferences {
        if !merged.preferences.contains(pref) {
            merged.preferences.push(pref.clone());
        }
    }
    
    // 更新 background（如果置信度高）
    if new_profile.confidence > 0.8 {
        if let Some(bg) = &new_profile.background {
            merged.background = Some(bg.clone());
        }
    }
    
    merged.updated_at = chrono::Utc::now();
    merged
}
```

### 2.3 JSON Schema 约束

使用 OpenAI 的 `response_format` 参数：
```rust
let request = ChatRequest {
    model: "gpt-4o-mini".to_string(),
    messages: vec![...],
    response_format: Some(ResponseFormat {
        type_: "json_schema".to_string(),
        json_schema: Some(JsonSchema {
            name: "user_profile".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "traits": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "preferences": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "background": { "type": "string" },
                    "knowledge": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "confidence": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1
                    }
                },
                "required": ["traits", "preferences", "knowledge", "confidence"]
            }),
            strict: true,
        }),
    }),
    ..Default::default()
};
```

### 2.4 评估数据集

创建评估数据集：
```jsonl
{"conversation": ["user: I'm a software engineer", "assistant: Great!"], "expected": {"traits": [], "preferences": [], "background": "software engineer", "knowledge": ["programming"]}}
{"conversation": ["user: I love Rust and open source", "assistant: Nice!"], "expected": {"traits": [], "preferences": ["Rust", "open source"], "background": null, "knowledge": []}}
```

评估脚本：
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_profile_extraction_accuracy() {
        let dataset = load_eval_dataset("docs/references/profile_eval_dataset.jsonl");
        let llm = OpenAiAdapter::new(...);
        
        let mut correct = 0;
        let mut total = 0;
        
        for case in dataset {
            let extracted = llm.extract_profile(&case.conversation).await.unwrap();
            if profiles_match(&extracted, &case.expected) {
                correct += 1;
            }
            total += 1;
        }
        
        let accuracy = correct as f64 / total as f64;
        assert!(accuracy >= 0.9, "Accuracy {} below threshold", accuracy);
    }
}
```

---

## 3. 实施计划

### Day 1-2: LLM 总结
- [ ] 实现 `summarize()` trait 方法
- [ ] 更新 `consolidate_to_mid_term_internal()`
- [ ] 添加质量评估
- [ ] 编写单元测试

### Day 3-4: Profile 提取
- [ ] 实现 `extract_profile()` trait 方法
- [ ] 实现增量更新逻辑
- [ ] 添加 JSON Schema 约束
- [ ] 创建评估数据集
- [ ] 编写单元测试

### Day 5: 集成测试
- [ ] 端到端测试
- [ ] 性能测试
- [ ] 文档更新

---

## 4. 验收标准

### 功能验收
- [ ] LLM 总结压缩比 < 0.5
- [ ] Profile 提取准确率 > 90%
- [ ] 支持增量更新
- [ ] 所有测试通过

### 性能验收
- [ ] 总结延迟 < 3s
- [ ] Profile 提取延迟 < 2s
- [ ] 不影响主流程性能

### 代码质量
- [ ] 代码覆盖率 > 80%
- [ ] 文档完善
- [ ] 错误处理完善
