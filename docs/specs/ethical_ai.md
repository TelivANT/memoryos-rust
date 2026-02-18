# Ethical AI & Bias Mitigation (P1)

> **Status**: Approved
> **Objective**: Prevent discriminatory content in memory storage and retrieval.

## 1. Problem Statement

**Risk**: AI systems can perpetuate and amplify human biases present in training data or user inputs.

**Example Scenarios**:
- HR records: "Female candidates are less technical"
- Hiring notes: "Older workers are slow learners"
- Performance reviews: "Asian employees are good at math but lack leadership"

**Impact**: Legal liability (discrimination lawsuits), reputational damage, ethical violations.

---

## 2. Bias Detection Framework

### 2.1 Protected Attributes

The system MUST detect and flag content related to:

| Category | Examples | Regex Patterns |
| :--- | :--- | :--- |
| **Gender** | male, female, woman, man, transgender | `\b(fe)?male\b`, `\bwom[ae]n\b` |
| **Race/Ethnicity** | Black, Asian, Hispanic, White | `\b(black\|asian\|hispanic)\b` |
| **Age** | old, young, elderly, millennial | `\b(old\|young\|elderly)\b` |
| **Religion** | Muslim, Christian, Jewish, Hindu | `\b(muslim\|christian\|jewish)\b` |
| **Disability** | disabled, handicapped, blind | `\b(disabled\|handicapped)\b` |
| **Sexual Orientation** | gay, lesbian, straight | `\b(gay\|lesbian\|lgbtq)\b` |

### 2.2 Bias Indicators (Red Flags)

**Pattern**: `[Protected Attribute] + [Negative Stereotype]`

Examples:
- "Women are not good at..."
- "Old people cannot..."
- "Muslims are likely to..."

**Detection Logic**:
```rust
fn detect_bias(text: &str) -> Option<BiasType> {
    let protected_attrs = ["women", "female", "old", "muslim", ...];
    let negative_verbs = ["cannot", "are not", "lack", "fail to", ...];
    
    for attr in protected_attrs {
        for verb in negative_verbs {
            if text.contains(attr) && text.contains(verb) {
                return Some(BiasType::Stereotype);
            }
        }
    }
    None
}
```

---

## 3. Mitigation Strategies

### 3.1 Strategy A: Content Filtering (Pre-Storage)

**Trigger**: Before storing any Fact/Profile to LTM

**Action**:
1. Run bias detection on extracted content
2. If bias detected:
   - **Reject storage**
   - Return error: `400 Bad Request - Potentially discriminatory content detected`
   - Log incident for audit

**Example**:
```json
{
  "error": {
    "code": "bias_detected",
    "message": "Content contains potential bias related to gender",
    "detected_phrase": "women are not good at",
    "suggestion": "Please rephrase without stereotypes"
  }
}
```

### 3.2 Strategy B: Human-in-the-Loop (HITL)

**Trigger**: High-risk domains (HR, Hiring, Performance Reviews)

**Action**:
1. Extract memory as usual
2. Mark as `pending_review`
3. Send to Admin Dashboard for approval
4. Only store if approved

**Workflow**:
```
User Input → LLM Extraction → Bias Check
                                  ↓
                            Bias Detected?
                                  ↓
                            Yes → Review Queue
                                  ↓
                            Admin Approves? → Store
                            Admin Rejects? → Discard
```

### 3.3 Strategy C: Contextual Allowlist

**Problem**: Not all mentions of protected attributes are biased.

**Example (Legitimate)**:
- "We need to hire more women in engineering" (Diversity goal)
- "Provide wheelchair access for disabled employees" (Accommodation)

**Solution**: Allowlist patterns
```rust
const ALLOWLIST: &[&str] = &[
    "hire more women",
    "diversity initiative",
    "accommodate disabled",
    "equal opportunity",
];

fn is_allowlisted(text: &str) -> bool {
    ALLOWLIST.iter().any(|pattern| text.contains(pattern))
}
```

---

## 4. Fairness Auditing

### 4.1 Periodic Memory Audit

**Frequency**: Monthly

**Process**:
1. Sample 1000 random memories from LTM
2. Run bias detection on each
3. Calculate bias prevalence:
   ```
   bias_rate = (biased_memories / total_memories) * 100
   ```
4. Target: < 0.1% (1 in 1000)

**Report**:
```markdown
## Bias Audit Report - February 2026

- Total Memories Audited: 1,000
- Biased Content Detected: 2 (0.2%)
- Categories:
  - Gender: 1
  - Age: 1
- Action Taken: Flagged for manual review
```

### 4.2 Disparate Impact Analysis

**Goal**: Ensure memory retrieval does not favor/disfavor certain groups.

**Method**:
1. Simulate queries from different personas:
   - Persona A: "Female software engineer"
   - Persona B: "Male software engineer"
2. Compare retrieved memories
3. Measure similarity: Should be > 95%

**Alert**: If similarity < 90% → Investigate bias in retrieval algorithm

---

## 5. User Controls

### 5.1 Bias Sensitivity Setting

**API**: `PUT /v1/user/settings`
```json
{
  "bias_detection_level": "strict"  // "off" | "moderate" | "strict"
}
```

**Levels**:
- **Off**: No bias detection (not recommended)
- **Moderate**: Detect obvious stereotypes
- **Strict**: Detect any mention of protected attributes in negative context

### 5.2 Memory Flagging (User-Initiated)

**API**: `POST /v1/memory/{id}/flag`
```json
{
  "reason": "bias",
  "details": "This memory contains gender stereotypes"
}
```

**Action**:
- Mark memory as `flagged`
- Send to review queue
- If 3+ users flag same memory → Auto-deprecate

---

## 6. Training Data Curation

### 6.1 FAQ Seeding (Admin Import)

**Problem**: Initial FAQ data may contain historical biases.

**Solution**: Pre-import audit
1. Admin prepares `faq.csv`
2. Run bias detection on all entries
3. Reject biased entries
4. Require manual review for borderline cases

**Example**:
```csv
question,answer,bias_score
"How to reset password?","Contact IT at ext 1234",0.0
"Are women good at coding?","[REJECTED - Bias detected]",0.95
```

### 6.2 Synthetic Data Augmentation

**Goal**: Balance training data to reduce bias.

**Method**:
1. Identify underrepresented groups in memory
2. Generate synthetic examples:
   - "Female engineer solved critical bug"
   - "Senior developer (age 60) mentored team"
3. Add to LTM with `source: synthetic`

---

## 7. Transparency & Explainability

### 7.1 Bias Detection Logs

**Storage**: Separate audit table
```sql
CREATE TABLE bias_incidents (
    id UUID PRIMARY KEY,
    user_id VARCHAR(255),
    detected_at TIMESTAMP,
    content_hash VARCHAR(64),  -- SHA256 of content
    bias_type VARCHAR(50),     -- gender, race, age, etc.
    action_taken VARCHAR(50),  -- rejected, flagged, approved
    reviewer_id VARCHAR(255)   -- If HITL
);
```

**Retention**: 2 years (compliance)

### 7.2 User Notification

**If content rejected**:
```
⚠️ Your message was not stored in memory.

Reason: Potential bias detected related to [gender].

Detected phrase: "women are not good at"

Suggestion: Please rephrase without stereotypes. Example:
"Some individuals may need additional training in..."

Learn more: https://memoryos.com/docs/bias-policy
```

---

## 8. Legal & Compliance

### 8.1 Anti-Discrimination Laws

**Applicable Regulations**:
- **US**: Title VII (Civil Rights Act), EEOC Guidelines
- **EU**: GDPR Article 22 (Automated Decision-Making)
- **UK**: Equality Act 2010

**Compliance**:
- ✅ Bias detection implemented
- ✅ Human review for high-risk decisions
- ✅ Audit trail maintained
- ✅ User right to explanation

### 8.2 AI Ethics Board

**Composition**:
- 1 Legal Counsel
- 1 Ethicist
- 1 Diversity & Inclusion Officer
- 2 Engineers

**Responsibilities**:
- Review bias incidents quarterly
- Update detection rules
- Approve policy changes

---

## 9. Incident Response

### 9.1 Scenario: Biased Memory Discovered in Production

**Timeline**:
- **T+0**: User reports biased memory via `/flag` API
- **T+1h**: Admin reviews and confirms bias
- **T+2h**: Memory deprecated (`is_active: false`)
- **T+4h**: Root cause analysis (How did it pass detection?)
- **T+24h**: Update detection rules
- **T+48h**: Re-audit all memories with new rules

### 9.2 Public Disclosure

**If bias affects > 100 users**:
1. Publish incident report on status page
2. Notify affected users via email
3. Offer free bias audit of their memories
4. Implement corrective measures

---

## 10. Metrics & KPIs

### 10.1 Bias Detection Metrics

| Metric | Target | Current | Trend |
| :--- | :--- | :--- | :--- |
| **Bias Detection Rate** | < 0.1% | 0.05% | ✅ |
| **False Positive Rate** | < 5% | 3% | ✅ |
| **HITL Review Time** | < 24h | 18h | ✅ |
| **User Flags per 1000 Memories** | < 1 | 0.8 | ✅ |

### 10.2 Fairness Metrics

**Demographic Parity**:
```
P(Positive Outcome | Group A) ≈ P(Positive Outcome | Group B)
```

**Example**: Retrieval success rate should be equal across genders.

---

## 11. Future Enhancements

### 11.1 ML-Based Bias Detection

**Current**: Rule-based (regex)
**Future**: Fine-tuned BERT model for bias classification

**Benefits**:
- Detect subtle biases (e.g., "articulate" used only for minorities)
- Multilingual support
- Lower false positive rate

### 11.2 Counterfactual Fairness

**Method**: Test if changing protected attribute changes outcome

**Example**:
- Query: "Recommend a candidate for promotion"
- Test 1: Candidate is "John" (male)
- Test 2: Candidate is "Jane" (female)
- If recommendations differ → Bias detected

---

## 12. Training & Awareness

### 12.1 Mandatory Training

**All employees MUST complete**:
- "Unconscious Bias in AI" (2 hours)
- "Ethical AI Development" (1 hour)
- Annual refresher

### 12.2 Bias Incident Drills

**Quarterly Exercise**:
1. Inject synthetic biased memory
2. Measure detection time
3. Measure response time
4. Update runbook

---

## 13. External Resources

- **NIST AI Risk Management Framework**: https://www.nist.gov/itl/ai-risk-management-framework
- **EU AI Act**: High-risk AI systems requirements
- **Google's PAIR Guidebook**: https://pair.withgoogle.com/guidebook/
- **Microsoft's Responsible AI**: https://www.microsoft.com/en-us/ai/responsible-ai
