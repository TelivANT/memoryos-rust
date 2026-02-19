# Configuration Examples

This directory contains ready-to-use configuration templates for different deployment scenarios.

## 📁 Available Configurations

### 1. `config.basic.toml` - Basic Setup
**Use Case**: Development, Testing, Small-scale deployment

**Features**:
- Single OpenAI provider
- Local Redis and Qdrant
- Minimal configuration
- Auth disabled

**Quick Start**:
```bash
cp examples/config.basic.toml config.toml
# Edit API keys
cargo run --release
```

---

### 2. `config.production.toml` - Production Ready
**Use Case**: Production deployment, High availability

**Features**:
- Multiple LLM providers (Gemini + OpenAI)
- Redis Cluster support
- Smart routing enabled
- API key authentication
- Enhanced security

**Quick Start**:
```bash
cp examples/config.production.toml config.toml
# Configure Redis Cluster and API keys
cargo run --release
```

---

### 3. `config.high-performance.toml` - Maximum Speed
**Use Case**: High-throughput scenarios, 10K+ QPS

**Features**:
- 16 worker threads
- Aggressive local routing (FAQ <10ms)
- Fast LLM models (gpt-3.5-turbo)
- Optimized Redis settings
- Auth disabled for benchmarking

**Performance**:
- FAQ: <10ms, 50K QPS
- Hybrid: 100ms avg, 15K QPS

**Quick Start**:
```bash
cp examples/config.high-performance.toml config.toml
# Tune worker_threads to match CPU cores
cargo run --release
```

---

### 4. `config.cost-optimized.toml` - 85-90% Cost Savings
**Use Case**: Budget-conscious deployments

**Features**:
- Local Llama as primary (Free)
- DeepSeek as fallback (Cheap)
- OpenAI for complex queries only
- Smart routing for cost optimization

**Cost Breakdown**:
- 60% FAQ Direct Hit: $0
- 30% Local Llama: $0
- 8% DeepSeek: ~$0.001/1K tokens
- 2% OpenAI: ~$0.03/1K tokens

**Total Savings**: 85-90% vs pure OpenAI

**Quick Start**:
```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3:8b

# Configure MemoryOS
cp examples/config.cost-optimized.toml config.toml
cargo run --release
```

---

### 5. `config.local-llm.toml` - Privacy First
**Use Case**: Privacy-sensitive, Air-gapped environments

**Features**:
- 100% local inference (Ollama/vLLM)
- No external API calls
- GDPR/HIPAA compliant
- Local embedding models

**Privacy Benefits**:
- Data never leaves your infrastructure
- Air-gapped deployment supported
- Full control over models

**Hardware Requirements**:
- llama3:8b: 8GB VRAM (RTX 3060 Ti)
- llama3:70b: 48GB VRAM (A100) or 64GB RAM

**Quick Start**:
```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3:70b
ollama pull nomic-embed-text

# Configure MemoryOS
cp examples/config.local-llm.toml config.toml
cargo run --release
```

---

## 🔧 Configuration Tips

### Choosing the Right Config

| Scenario | Recommended Config | Expected Performance |
|----------|-------------------|---------------------|
| Development | `basic` | Good enough |
| Production (Cloud) | `production` | High availability |
| High Traffic | `high-performance` | 10K+ QPS |
| Budget Limited | `cost-optimized` | 85-90% savings |
| Privacy Required | `local-llm` | 100% private |

### Common Customizations

#### 1. Change LLM Provider
```toml
[llm]
default_provider = "gemini"  # or "openai", "ollama", "deepseek"
```

#### 2. Enable Smart Routing
```toml
[router]
enable = true
hot_threshold = 0.8  # 0.0-1.0, higher = more local routing
```

#### 3. Configure Redis Cluster
```toml
[storage.redis]
url = "redis://node1:6379,node2:6379,node3:6379"
```

#### 4. Add More LLM Providers
```toml
[llm.providers.claude]
type = "openai"
api_key = "<YOUR_ANTHROPIC_API_KEY>"
base_url = "https://api.anthropic.com/v1"
```

---

## 📚 Related Documentation

- [Quick Start Guide](../docs/QUICKSTART.md)
- [Configuration Reference](../docs/ops/config_reference.md)
- [Deployment Guide](../docs/DEPLOYMENT.md)
- [Performance Tuning](../docs/DESIGN.md)

---

## 🆘 Need Help?

- **GitHub Issues**: [Report Issues](https://github.com/TelivANT/memoryos-rust/issues)
- **GitHub Discussions**: [Ask Questions](https://github.com/TelivANT/memoryos-rust/discussions)
- **Documentation**: [Read the Docs](../docs/README.md)
