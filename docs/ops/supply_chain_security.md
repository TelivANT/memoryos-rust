# Supply Chain Security (P1)

> **Status**: Approved
> **Objective**: Prevent dependency poisoning and ensure software integrity.

## 1. Dependency Management Policy

### 1.1 Version Pinning (Mandatory)

**Cargo.toml Requirements**:
```toml
[dependencies]
# ❌ BAD: Allows automatic minor/patch updates
tokio = "1.35"

# ✅ GOOD: Exact version pinning
tokio = "=1.35.1"
axum = "=0.7.4"
redis = "=0.32.7"
```

**Rationale**: Prevents supply chain attacks via malicious patch releases.

### 1.2 Cargo.lock Commitment
- **MUST** commit `Cargo.lock` to Git
- **MUST** review `Cargo.lock` changes in PR
- **MUST NOT** use `cargo update` without security review

---

## 2. Dependency Vetting Process

### 2.1 New Dependency Checklist

Before adding any new crate, verify:

| Criterion | Threshold | Tool |
| :--- | :--- | :--- |
| **GitHub Stars** | > 500 | Manual check |
| **Recent Activity** | Commit in last 6 months | GitHub |
| **Known Vulnerabilities** | 0 critical/high | `cargo audit` |
| **Maintainer Reputation** | Verified identity | GitHub profile |
| **License Compatibility** | Apache-2.0 / MIT | `cargo license` |
| **Code Review** | Manual audit (critical deps) | Human |

**Critical Dependencies** (require full code audit):
- Cryptography: `ring`, `rustls`
- Serialization: `serde`, `serde_json`
- Network: `tokio`, `hyper`, `reqwest`

### 2.2 Approval Process
1. Engineer proposes new dependency (GitHub Issue)
2. Security team reviews (2 business days)
3. If approved → Add to `approved_dependencies.txt`
4. If rejected → Find alternative or implement in-house

---

## 3. Automated Security Scanning

### 3.1 CI/CD Integration

**GitHub Actions Workflow** (`.github/workflows/security.yml`):
```yaml
name: Security Audit
on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      # 1. Check for known vulnerabilities
      - name: Cargo Audit
        run: |
          cargo install cargo-audit
          cargo audit --deny warnings
      
      # 2. Check for outdated dependencies
      - name: Cargo Outdated
        run: |
          cargo install cargo-outdated
          cargo outdated --exit-code 1
      
      # 3. License compliance
      - name: Cargo Deny
        run: |
          cargo install cargo-deny
          cargo deny check licenses
      
      # 4. SBOM generation
      - name: Generate SBOM
        run: |
          cargo install cargo-sbom
          cargo sbom > sbom.json
      
      - name: Upload SBOM
        uses: actions/upload-artifact@v3
        with:
          name: sbom
          path: sbom.json
```

### 3.2 Vulnerability Database
- **Source**: RustSec Advisory Database
- **Update Frequency**: Daily (automated)
- **Action on Detection**: 
  - Critical/High → Block deployment
  - Medium → Create Jira ticket
  - Low → Log warning

---

## 4. Private Dependency Registry (Enterprise)

### 4.1 Architecture

```
Developer → Push to Internal GitLab
              ↓
         Security Scan (Automated)
              ↓
         Approved? → Private Cargo Registry
              ↓
         Production Build (Only uses private registry)
```

### 4.2 Implementation (Artifactory)

**Cargo Config** (`~/.cargo/config.toml`):
```toml
[source.crates-io]
replace-with = "company-registry"

[source.company-registry]
registry = "https://cargo.company.internal/api/cargo"
```

**Benefits**:
- All dependencies pre-vetted
- Immune to crates.io outages
- Audit trail of all dependency usage

---

## 5. Build Reproducibility

### 5.1 Deterministic Builds

**Goal**: Same source code → Same binary (bit-for-bit)

**Requirements**:
- Fixed Rust version: `rust-toolchain.toml`
  ```toml
  [toolchain]
  channel = "1.75.0"
  components = ["rustfmt", "clippy"]
  ```
- Fixed build flags: `.cargo/config.toml`
  ```toml
  [build]
  rustflags = ["-C", "link-arg=-Wl,--build-id=sha1"]
  ```

**Verification**:
```bash
# Build twice
cargo clean && cargo build --release
sha256sum target/release/memoryos-rust > hash1.txt

cargo clean && cargo build --release
sha256sum target/release/memoryos-rust > hash2.txt

# Compare
diff hash1.txt hash2.txt
# Expected: No difference
```

### 5.2 SLSA Level 3 Compliance

**Requirements**:
- ✅ Source integrity (Git signed commits)
- ✅ Build integrity (Reproducible builds)
- ✅ Provenance (SBOM + build logs)

---

## 6. Runtime Integrity Verification

### 6.1 Binary Signing (Release)

**Process**:
1. Build binary in CI
2. Sign with GPG key: `gpg --detach-sign memoryos-rust`
3. Upload binary + signature to GitHub Releases
4. Users verify: `gpg --verify memoryos-rust.sig memoryos-rust`

**Public Key Distribution**:
```bash
# Import MemoryOS public key
curl https://memoryos.com/gpg-key.asc | gpg --import
```

### 6.2 Container Image Signing (Cosign)

**Sign Docker Image**:
```bash
# Build
docker build -t memoryos-rust:v1.0.0 .

# Sign with Sigstore
cosign sign --key cosign.key memoryos-rust:v1.0.0
```

**Verify Before Deploy**:
```bash
cosign verify --key cosign.pub memoryos-rust:v1.0.0
```

---

## 7. Incident Response Plan

### 7.1 Scenario: Dependency Vulnerability Disclosed

**Timeline**:
- **T+0**: RustSec publishes advisory (e.g., `tokio` RCE)
- **T+1h**: CI fails with `cargo audit` error
- **T+2h**: Security team assesses impact
- **T+4h**: Patch available (upgrade `tokio`)
- **T+6h**: Deploy to production
- **T+24h**: Post-mortem published

### 7.2 Scenario: Malicious Dependency Detected

**Example**: `redis-rs 0.32.8` contains backdoor

**Response**:
1. **Immediate**: Rollback to last known good version (`0.32.7`)
2. **Isolate**: Block all network egress from affected Pods
3. **Audit**: Check logs for suspicious activity
4. **Rotate**: All API keys and secrets
5. **Notify**: Users via email + status page
6. **Report**: File CVE + notify RustSec

---

## 8. Developer Workstation Security

### 8.1 Mandatory Tools

**Pre-commit Hooks** (`.git/hooks/pre-commit`):
```bash
#!/bin/bash
# Run security checks before commit
cargo audit || exit 1
cargo clippy -- -D warnings || exit 1
```

### 8.2 Environment Isolation

**Use Docker for Development**:
```dockerfile
FROM rust:1.75-slim
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked
COPY . .
RUN cargo build --release
```

**Benefits**:
- Isolated from host system
- Reproducible environment
- Prevents "works on my machine"

---

## 9. Third-Party Audit

### 9.1 Annual Security Audit

**Scope**:
- Dependency review (all 100+ crates)
- Code review (critical paths)
- Penetration testing

**Vendor**: Trail of Bits / NCC Group

**Deliverable**: 
- Audit report (public)
- Remediation plan (internal)

### 9.2 Bug Bounty Program

**Platform**: HackerOne

**Scope**:
- Supply chain attacks
- Dependency confusion
- Malicious code injection

**Rewards**:
- Critical: $5,000
- High: $2,000
- Medium: $500

---

## 10. Compliance Mapping

### 10.1 NIST SSDF (Secure Software Development Framework)

| Practice | Implementation |
| :--- | :--- |
| **PO.3.1**: Use version control | ✅ Git + signed commits |
| **PO.3.2**: Review dependencies | ✅ Manual + automated audit |
| **PS.1.1**: Secure build pipeline | ✅ Isolated CI runners |
| **PS.3.1**: Verify integrity | ✅ Binary signing + SBOM |

### 10.2 OWASP Top 10 (A06:2021 - Vulnerable Components)

**Mitigation**:
- ✅ Automated scanning (`cargo audit`)
- ✅ Dependency pinning
- ✅ Private registry (enterprise)
- ✅ Incident response plan

---

## 11. Metrics & KPIs

### 11.1 Security Posture Metrics

| Metric | Target | Current | Trend |
| :--- | :--- | :--- | :--- |
| **Known Vulnerabilities** | 0 | 0 | ✅ |
| **Outdated Dependencies** | < 5% | 2% | ✅ |
| **Unvetted Dependencies** | 0 | 0 | ✅ |
| **SBOM Coverage** | 100% | 100% | ✅ |

### 11.2 Dashboards

**Grafana Panel**:
- `cargo_audit_vulnerabilities_total` (Gauge)
- `dependency_age_days` (Histogram)
- `sbom_generation_success` (Counter)

---

## 12. Training & Awareness

### 12.1 Onboarding Checklist

New engineers MUST complete:
- [ ] Read this document
- [ ] Complete "Secure Coding in Rust" course
- [ ] Pass supply chain security quiz (80% score)
- [ ] Review recent security incidents

### 12.2 Quarterly Security Drills

**Scenario**: Simulate malicious dependency
1. Inject fake vulnerability in test environment
2. Measure detection time
3. Measure response time
4. Update runbook based on learnings
