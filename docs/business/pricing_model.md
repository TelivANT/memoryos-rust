# Pricing & Fair Use Policy (P0)

> **Status**: Approved
> **Objective**: Ensure profitability and prevent abuse.

## 1. Pricing Tiers

### Tier 1: Free (Personal / Developer)
*   **Target**: Individual developers, small hobby projects.
*   **Cost**: $0 / month.
*   **Limits**:
    *   1,000 messages / month.
    *   **Local Models Only** (Ollama).
    *   No Cloud LLM access (unless BYO Key).
    *   No Wiki Export.

### Tier 2: Pro (Team)
*   **Target**: Small teams, startups.
*   **Cost**: $29 / user / month.
*   **Limits**:
    *   10,000 messages / user / month.
    *   **Cloud Models**: Access to GPT-4o / Gemini Pro.
    *   **Storage**: 1GB Vector Storage.
    *   **Wiki Export**: S3 only.

### Tier 3: Enterprise (Corp)
*   **Target**: Large organizations (> 100 users).
*   **Cost**: Custom pricing.
*   **Features**:
    *   Unlimited messages (Volume pricing).
    *   **Dedicated Instance**: Private Qdrant/Redis cluster.
    *   **SLA**: 99.9% Uptime.
    *   **Compliance**: SSO, Audit Logs, GDPR DPA.

## 2. Fair Use Policy (FUP)

To prevent "Token Arbitrage" (User paying $29 but consuming $100 in tokens).

### 2.1 Over-Quota Handling
*   **Warning**: At 80% usage -> Email alert.
*   **Soft Limit**: At 100% usage -> Downgrade to **Local Models Only**.
*   **Hard Limit**: At 200% usage -> Block API access until next billing cycle or top-up.

### 2.2 Abuse Detection
*   **QPS Throttling**: Max 10 requests/sec per user.
*   **Bot Detection**: If > 1000 messages in 1 hour -> Trigger CAPTCHA or Admin Review.
*   **Account Sharing**: If 1 account accesses from > 5 IPs in 1 hour -> Force logout.
