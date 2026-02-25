# AI Agent Guidelines & Engineering Standards

**Version**: 1.0.0  
**Purpose**: Universal red/yellow line guidelines for AI agents and human engineers across all roles  
**Scope**: Open-source, language-agnostic, role-specific best practices

---

## 📋 Table of Contents

1. [How to Use This Document](#how-to-use-this-document)
2. [Architecture](#1-architect)
3. [Security Engineering](#2-security-engineer)
4. [Testing & QA](#3-test-engineer)
5. [Database Administration](#4-database-administrator-dba)
6. [Backend Engineering](#5-backend-engineers)
   - [Rust](#rust-engineer)
   - [Go](#go-engineer)
   - [Java](#java-engineer)
   - [Python](#python-engineer)
   - [C/C++](#cc-engineer)
   - [Node.js/TypeScript](#nodejstypescript-engineer)
   - [PHP](#php-engineer)
   - [Ruby](#ruby-engineer)
7. [Frontend Engineering](#6-frontend-engineers)
   - [React](#react-engineer)
   - [Vue](#vue-engineer)
   - [Angular](#angular-engineer)
   - [Svelte](#svelte-engineer)
8. [Mobile Engineering](#7-mobile-engineers)
   - [iOS/Swift](#iosswift-engineer)
   - [Android/Kotlin](#androidkotlin-engineer)
   - [Flutter/Dart](#flutterdart-engineer)
   - [React Native](#react-native-engineer)
9. [DevOps & SRE](#8-devopssre-engineer)
10. [Algorithm Engineering](#9-algorithm-engineer)
11. [Product Management](#10-product-manager)
12. [Technical Writing](#11-technical-writer)
13. [Code Review](#12-code-reviewer)

---

## How to Use This Document

### 🔴 Red Lines (MUST NOT)
**Absolute prohibitions.** Violating these causes:
- Security vulnerabilities
- Data loss
- System crashes
- Legal/compliance issues

**Action**: Block PR, reject code, escalate immediately.

### 🟡 Yellow Lines (SHOULD NOT)
**Strong warnings.** Violating these causes:
- Technical debt
- Performance degradation
- Maintenance burden
- Poor user experience

**Action**: Request changes, document exceptions, require approval.

### ✅ Green Principles (MUST DO)
**Core best practices.** Following these ensures:
- Code quality
- System reliability
- Team productivity
- Long-term maintainability

---

## 1. Architect

### 🔴 Red Lines (MUST NOT)

1. **NEVER introduce circular dependencies between modules**
   - ❌ `module_a` imports `module_b`, `module_b` imports `module_a`
   - ✅ Use dependency inversion (interfaces/traits)

2. **NEVER hardcode external service URLs/credentials in code**
   - ❌ `const API_URL = "https://api.example.com"`
   - ✅ Use environment variables + config management

3. **NEVER design single points of failure without fallback**
   - ❌ Single Redis instance with no replica
   - ✅ Cluster mode + circuit breaker + graceful degradation

4. **NEVER expose internal implementation details in public APIs**
   - ❌ `/api/v1/database/users/raw_query`
   - ✅ `/api/v1/users` (abstract implementation)

5. **NEVER allow unbounded resource consumption**
   - ❌ No pagination, no rate limiting, no timeout
   - ✅ Pagination + rate limits + request timeouts

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid premature optimization before profiling**
   - Optimize hot paths only after measurement

2. **Avoid tight coupling between layers**
   - Use ports/adapters pattern for external dependencies

3. **Avoid mixing business logic with infrastructure code**
   - Keep domain logic pure, infrastructure at edges

4. **Avoid creating new frameworks when existing ones suffice**
   - Prefer battle-tested libraries over custom solutions

5. **Avoid breaking changes in public APIs without versioning**
   - Use `/v1`, `/v2` or feature flags for migrations

### ✅ Green Principles (MUST DO)

1. **Design for horizontal scalability from day one**
2. **Document architectural decisions (ADRs)**
3. **Define clear module boundaries and ownership**
4. **Plan for observability (logs, metrics, traces)**
5. **Review dependency graph regularly for complexity**

---

## 2. Security Engineer

### 🔴 Red Lines (MUST NOT)

1. **NEVER store passwords/secrets in plaintext**
   - ❌ `password = "admin123"` in database
   - ✅ bcrypt/argon2 hashing + salt

2. **NEVER trust user input without validation**
   - ❌ `query = f"SELECT * FROM users WHERE id={user_input}"`
   - ✅ Parameterized queries + input sanitization

3. **NEVER expose stack traces/internal errors to users**
   - ❌ `500 Internal Server Error: NullPointerException at line 42`
   - ✅ Generic error message + log details server-side

4. **NEVER use weak cryptography (MD5, SHA1, DES)**
   - ❌ `md5(password)`
   - ✅ AES-256-GCM, SHA-256, bcrypt

5. **NEVER disable security features in production**
   - ❌ `CORS allow_origin = "*"` with credentials
   - ✅ Whitelist specific origins

6. **NEVER log sensitive data (PII, credentials, tokens)**
   - ❌ `logger.info(f"User {email} logged in with password {pwd}")`
   - ✅ `logger.info(f"User {user_id} logged in")`

7. **NEVER use default/hardcoded credentials**
   - ❌ `admin/admin`, `root/root`
   - ✅ Force password change on first login

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using deprecated security libraries**
   - Check CVE databases regularly

2. **Avoid storing sensitive data longer than necessary**
   - Implement data retention policies

3. **Avoid custom authentication/authorization logic**
   - Use proven frameworks (OAuth2, JWT, RBAC)

4. **Avoid exposing detailed API error messages**
   - Return generic errors, log details internally

5. **Avoid running services as root/admin**
   - Use least-privilege principle

### ✅ Green Principles (MUST DO)

1. **Implement defense in depth (multiple security layers)**
2. **Conduct regular security audits and penetration testing**
3. **Use constant-time comparison for secrets**
4. **Implement rate limiting and DDoS protection**
5. **Encrypt data at rest and in transit (TLS 1.3+)**
6. **Maintain security incident response plan**

---

## 3. Test Engineer

### 🔴 Red Lines (MUST NOT)

1. **NEVER commit code without any tests**
   - ❌ 0% test coverage on new features
   - ✅ Minimum 70% coverage for critical paths

2. **NEVER use production data in tests**
   - ❌ Connect to prod database in test suite
   - ✅ Use test fixtures, mocks, or anonymized data

3. **NEVER write flaky tests (non-deterministic)**
   - ❌ Tests that randomly fail due to timing/race conditions
   - ✅ Deterministic tests with proper synchronization

4. **NEVER skip tests in CI/CD pipeline**
   - ❌ `--skip-tests` flag in production builds
   - ✅ All tests must pass before merge

5. **NEVER test implementation details instead of behavior**
   - ❌ Assert internal variable values
   - ✅ Test public API contracts and outcomes

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid writing tests that depend on execution order**
   - Each test should be independent

2. **Avoid excessive mocking (test becomes meaningless)**
   - Balance between unit and integration tests

3. **Avoid testing third-party library internals**
   - Test your code's integration with libraries

4. **Avoid long-running tests in unit test suite**
   - Move slow tests to integration/E2E suite

5. **Avoid hardcoded test data without explanation**
   - Use factories or builders for test data

### ✅ Green Principles (MUST DO)

1. **Write tests before fixing bugs (TDD for bug fixes)**
2. **Test edge cases and boundary conditions**
3. **Maintain separate test suites (unit/integration/E2E)**
4. **Use descriptive test names (what/when/expected)**
5. **Keep test code as clean as production code**
6. **Run tests locally before pushing**

---

## 4. Database Administrator (DBA)

### 🔴 Red Lines (MUST NOT)

1. **NEVER run DDL (ALTER/DROP) without backup**
   - ❌ `DROP TABLE users;` in production
   - ✅ Backup + dry-run + rollback plan

2. **NEVER use `SELECT *` in production queries**
   - ❌ `SELECT * FROM large_table`
   - ✅ `SELECT id, name, email FROM users`

3. **NEVER store large BLOBs in relational databases**
   - ❌ Store 10MB images in MySQL
   - ✅ Use object storage (S3) + store references

4. **NEVER delete data without soft-delete or archive**
   - ❌ `DELETE FROM orders WHERE created_at < '2020-01-01'`
   - ✅ `UPDATE orders SET deleted_at = NOW() WHERE ...`

5. **NEVER expose database directly to public internet**
   - ❌ PostgreSQL port 5432 open to 0.0.0.0/0
   - ✅ Private network + bastion host

6. **NEVER use weak database passwords**
   - ❌ `postgres/postgres`
   - ✅ Strong passwords + rotate regularly

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid N+1 query problems**
   - Use JOINs or batch loading

2. **Avoid missing indexes on frequently queried columns**
   - Profile slow queries regularly

3. **Avoid storing JSON blobs when relational schema fits**
   - Use proper normalization when possible

4. **Avoid long-running transactions**
   - Keep transactions short to avoid lock contention

5. **Avoid using database as message queue**
   - Use dedicated message brokers (Redis, RabbitMQ)

### ✅ Green Principles (MUST DO)

1. **Always use connection pooling**
2. **Monitor query performance and slow query logs**
3. **Implement automated backups with tested restore**
4. **Use read replicas for read-heavy workloads**
5. **Document schema changes with migrations**
6. **Set up monitoring and alerting for DB health**

---

## 5. Backend Engineers

### Common Backend Red Lines (All Languages)

1. **NEVER return sensitive data in API responses**
   - ❌ Include `password_hash` in user JSON
   - ✅ Exclude sensitive fields explicitly

2. **NEVER use blocking I/O in async contexts**
   - ❌ `time.sleep()` in async function
   - ✅ `await asyncio.sleep()`

3. **NEVER ignore error handling**
   - ❌ Empty `catch` blocks
   - ✅ Log errors, return proper status codes

4. **NEVER use global mutable state**
   - ❌ `global counter; counter += 1`
   - ✅ Thread-safe state management

5. **NEVER hardcode business logic in controllers**
   - ❌ Complex calculations in HTTP handlers
   - ✅ Separate business logic into services

---

### Rust Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `unwrap()` or `expect()` in production code**
   - ❌ `file.read_to_string().unwrap()`
   - ✅ `file.read_to_string()?` or proper error handling

2. **NEVER use `unsafe` without thorough documentation**
   - ❌ `unsafe { ... }` without safety invariants
   - ✅ Document why unsafe is needed + safety proof

3. **NEVER clone large data structures unnecessarily**
   - ❌ `let data = huge_vec.clone()`
   - ✅ Use references `&huge_vec` or `Arc<T>`

4. **NEVER use `Arc<Mutex<T>>` when `Arc<RwLock<T>>` fits**
   - ❌ Mutex for read-heavy workloads
   - ✅ RwLock for multiple readers

5. **NEVER ignore Clippy warnings without reason**
   - ❌ `#[allow(clippy::all)]`
   - ✅ Fix warnings or document exceptions

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid excessive `.clone()` calls**
2. **Avoid blocking operations in async functions**
3. **Avoid large enum variants (use `Box<T>`)**
4. **Avoid `String` when `&str` suffices**
5. **Avoid panics in library code**

#### ✅ Green Principles (MUST DO)

1. **Use `Result<T, E>` for fallible operations**
2. **Leverage type system for compile-time guarantees**
3. **Write unit tests with `#[cfg(test)]`**
4. **Use `cargo clippy` and `cargo fmt`**
5. **Document public APIs with `///` doc comments**

---

### Go Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER ignore errors**
   - ❌ `result, _ := doSomething()`
   - ✅ `if err != nil { return err }`

2. **NEVER use `panic()` for normal error handling**
   - ❌ `panic("invalid input")`
   - ✅ `return fmt.Errorf("invalid input")`

3. **NEVER forget to close resources**
   - ❌ `file, _ := os.Open("file.txt")`
   - ✅ `defer file.Close()`

4. **NEVER use goroutines without proper synchronization**
   - ❌ Shared mutable state without mutex
   - ✅ Use channels or `sync.Mutex`

5. **NEVER use `init()` for complex initialization**
   - ❌ Database connections in `init()`
   - ✅ Explicit initialization functions

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid goroutine leaks (always ensure termination)**
2. **Avoid using `interface{}` when type is known**
3. **Avoid deeply nested error handling**
4. **Avoid global variables**
5. **Avoid ignoring context cancellation**

#### ✅ Green Principles (MUST DO)

1. **Always handle errors explicitly**
2. **Use `context.Context` for cancellation**
3. **Run `go vet` and `golangci-lint`**
4. **Write table-driven tests**
5. **Use `defer` for cleanup**

---

### Java Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER catch `Exception` or `Throwable` broadly**
   - ❌ `catch (Exception e) {}`
   - ✅ Catch specific exceptions

2. **NEVER use `System.out.println()` for logging**
   - ❌ `System.out.println("Error: " + e)`
   - ✅ Use SLF4J/Logback

3. **NEVER create threads manually in production**
   - ❌ `new Thread(() -> ...).start()`
   - ✅ Use `ExecutorService`

4. **NEVER use `==` for String comparison**
   - ❌ `if (str == "hello")`
   - ✅ `if (str.equals("hello"))`

5. **NEVER ignore `InterruptedException`**
   - ❌ `catch (InterruptedException e) {}`
   - ✅ Restore interrupt status or propagate

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using `null` (use `Optional<T>`)**
2. **Avoid mutable static fields**
3. **Avoid catching `NullPointerException`**
4. **Avoid using raw types (use generics)**
5. **Avoid finalizers (use try-with-resources)**

#### ✅ Green Principles (MUST DO)

1. **Use try-with-resources for AutoCloseable**
2. **Prefer immutability (final fields)**
3. **Use streams for collection processing**
4. **Write unit tests with JUnit 5**
5. **Use dependency injection (Spring/Guice)**

---

### Python Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `eval()` or `exec()` on user input**
   - ❌ `eval(user_input)`
   - ✅ Use `ast.literal_eval()` or proper parsing

2. **NEVER use bare `except:` clauses**
   - ❌ `except: pass`
   - ✅ `except SpecificException as e:`

3. **NEVER use mutable default arguments**
   - ❌ `def func(items=[]):`
   - ✅ `def func(items=None): items = items or []`

4. **NEVER use `import *`**
   - ❌ `from module import *`
   - ✅ `from module import specific_function`

5. **NEVER modify list while iterating**
   - ❌ `for item in items: items.remove(item)`
   - ✅ `items = [x for x in items if condition]`

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using `global` keyword**
2. **Avoid deeply nested code (max 3 levels)**
3. **Avoid using `os.system()` (use `subprocess`)**
4. **Avoid catching `Exception` broadly**
5. **Avoid using `__dict__` for attribute access**

#### ✅ Green Principles (MUST DO)

1. **Use type hints (PEP 484)**
2. **Follow PEP 8 style guide**
3. **Use virtual environments (venv/poetry)**
4. **Write docstrings for public functions**
5. **Use `black` and `ruff` for formatting/linting**

---

### C/C++ Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `gets()` or unsafe string functions**
   - ❌ `gets(buffer)`
   - ✅ `fgets(buffer, size, stdin)`

2. **NEVER dereference null pointers**
   - ❌ `*ptr` without null check
   - ✅ `if (ptr != NULL) { *ptr }`

3. **NEVER use `malloc()` without checking return value**
   - ❌ `char *buf = malloc(100);`
   - ✅ `if (buf == NULL) { handle_error(); }`

4. **NEVER mix `malloc/free` with `new/delete`**
   - ❌ `int *p = (int*)malloc(sizeof(int)); delete p;`
   - ✅ Use consistent allocation/deallocation

5. **NEVER use `strcpy()` or `strcat()` without bounds**
   - ❌ `strcpy(dest, src)`
   - ✅ `strncpy(dest, src, sizeof(dest))`

6. **NEVER return pointers to local variables**
   - ❌ `char* func() { char buf[10]; return buf; }`
   - ✅ Use heap allocation or static storage

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid manual memory management (use RAII in C++)**
2. **Avoid using raw pointers (use smart pointers)**
3. **Avoid using `goto` except for cleanup**
4. **Avoid using `union` for type punning**
5. **Avoid using `volatile` for synchronization**

#### ✅ Green Principles (MUST DO)

1. **Always initialize variables**
2. **Use `const` for immutable data**
3. **Use RAII for resource management (C++)**
4. **Run static analyzers (clang-tidy, cppcheck)**
5. **Use smart pointers (`unique_ptr`, `shared_ptr`)**
6. **Enable compiler warnings (`-Wall -Wextra`)**

---

### Node.js/TypeScript Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `eval()` on user input**
   - ❌ `eval(userCode)`
   - ✅ Use proper parsing/validation

2. **NEVER block the event loop**
   - ❌ `while(true) {}` or `fs.readFileSync()` in handlers
   - ✅ Use async APIs

3. **NEVER ignore promise rejections**
   - ❌ `promise.then(...)` without `.catch()`
   - ✅ `await promise` in try-catch or `.catch()`

4. **NEVER use `var` (use `const`/`let`)**
   - ❌ `var x = 10;`
   - ✅ `const x = 10;` or `let x = 10;`

5. **NEVER use `==` (use `===`)**
   - ❌ `if (x == "5")`
   - ✅ `if (x === 5)`

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid callback hell (use async/await)**
2. **Avoid using `any` type in TypeScript**
3. **Avoid mutating function parameters**
4. **Avoid using `require()` in TypeScript (use `import`)**
5. **Avoid using `process.exit()` in libraries**

#### ✅ Green Principles (MUST DO)

1. **Use TypeScript for type safety**
2. **Use ESLint and Prettier**
3. **Handle all promise rejections**
4. **Use environment variables for config**
5. **Write tests with Jest/Vitest**

---

### PHP Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `eval()` on user input**
   - ❌ `eval($_GET['code'])`
   - ✅ Never use eval with external data

2. **NEVER use `mysql_*` functions (deprecated)**
   - ❌ `mysql_query($sql)`
   - ✅ Use PDO or MySQLi with prepared statements

3. **NEVER trust `$_GET`, `$_POST`, `$_COOKIE` directly**
   - ❌ `$id = $_GET['id']; $sql = "SELECT * FROM users WHERE id=$id"`
   - ✅ Use prepared statements

4. **NEVER use `md5()` for passwords**
   - ❌ `md5($password)`
   - ✅ `password_hash($password, PASSWORD_BCRYPT)`

5. **NEVER disable error reporting in production**
   - ❌ `error_reporting(0)`
   - ✅ Log errors, don't display to users

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using global variables**
2. **Avoid using `@` error suppression**
3. **Avoid using `extract()` on user input**
4. **Avoid using `include/require` with user input**
5. **Avoid using `==` (use `===`)**

#### ✅ Green Principles (MUST DO)

1. **Use Composer for dependency management**
2. **Follow PSR standards (PSR-12 for code style)**
3. **Use prepared statements for SQL**
4. **Use type declarations (PHP 7+)**
5. **Write tests with PHPUnit**

---

### Ruby Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `eval()` on user input**
   - ❌ `eval(params[:code])`
   - ✅ Use proper parsing

2. **NEVER use `send()` with user-controlled method names**
   - ❌ `object.send(params[:method])`
   - ✅ Whitelist allowed methods

3. **NEVER use string interpolation in SQL**
   - ❌ `User.where("name = '#{name}'")`
   - ✅ `User.where(name: name)`

4. **NEVER rescue `Exception` broadly**
   - ❌ `rescue Exception`
   - ✅ `rescue StandardError` or specific exceptions

5. **NEVER use `system()` with user input**
   - ❌ `system("ls #{user_input}")`
   - ✅ Use `Open3` with proper escaping

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid monkey-patching core classes**
2. **Avoid using global variables (`$var`)**
3. **Avoid using `class_eval` or `instance_eval`**
4. **Avoid deeply nested blocks**
5. **Avoid using `return` in blocks**

#### ✅ Green Principles (MUST DO)

1. **Use Bundler for dependency management**
2. **Follow Ruby Style Guide**
3. **Use RuboCop for linting**
4. **Write tests with RSpec or Minitest**
5. **Use strong parameters in Rails**

---

## 6. Frontend Engineers

### Common Frontend Red Lines (All Frameworks)

1. **NEVER store sensitive data in localStorage/sessionStorage**
   - ❌ `localStorage.setItem('token', jwt)`
   - ✅ Use httpOnly cookies

2. **NEVER trust client-side validation alone**
   - ❌ Only validate on frontend
   - ✅ Always validate on backend

3. **NEVER use `innerHTML` with user input**
   - ❌ `element.innerHTML = userInput`
   - ✅ Use `textContent` or sanitize with DOMPurify

4. **NEVER hardcode API keys in frontend code**
   - ❌ `const API_KEY = "sk-..."`
   - ✅ Use backend proxy

5. **NEVER ignore accessibility (a11y)**
   - ❌ `<div onclick="...">`
   - ✅ Use semantic HTML + ARIA

---

### React Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER mutate state directly**
   - ❌ `this.state.count = 5`
   - ✅ `this.setState({ count: 5 })`

2. **NEVER use array index as key in lists**
   - ❌ `{items.map((item, i) => <div key={i}>)}`
   - ✅ `{items.map(item => <div key={item.id}>)}`

3. **NEVER forget to cleanup effects**
   - ❌ `useEffect(() => { subscribe() })`
   - ✅ `useEffect(() => { subscribe(); return () => unsubscribe() })`

4. **NEVER use `dangerouslySetInnerHTML` without sanitization**
   - ❌ `<div dangerouslySetInnerHTML={{__html: userInput}} />`
   - ✅ Sanitize with DOMPurify first

5. **NEVER create components inside render**
   - ❌ `function Parent() { function Child() {...} return <Child /> }`
   - ✅ Define components outside

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid prop drilling (use Context or state management)**
2. **Avoid using `any` type in TypeScript**
3. **Avoid inline function definitions in JSX**
4. **Avoid large component files (split into smaller)**
5. **Avoid using `useEffect` for derived state**

#### ✅ Green Principles (MUST DO)

1. **Use TypeScript for type safety**
2. **Use ESLint + Prettier**
3. **Write tests with React Testing Library**
4. **Use proper key props in lists**
5. **Memoize expensive computations (`useMemo`)**

---

### Vue Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER mutate props directly**
   - ❌ `props.value = newValue`
   - ✅ Emit event to parent

2. **NEVER use `v-html` with user input**
   - ❌ `<div v-html="userInput"></div>`
   - ✅ Sanitize or use `{{ }}` interpolation

3. **NEVER forget to unregister event listeners**
   - ❌ `mounted() { window.addEventListener(...) }`
   - ✅ `beforeUnmount() { window.removeEventListener(...) }`

4. **NEVER use array index as key in `v-for`**
   - ❌ `v-for="(item, index) in items" :key="index"`
   - ✅ `:key="item.id"`

5. **NEVER access `$refs` in computed properties**
   - ❌ `computed: { value() { return this.$refs.input.value } }`
   - ✅ Use reactive data

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid deeply nested component trees**
2. **Avoid using `$parent` or `$children`**
3. **Avoid mixing Options API and Composition API**
4. **Avoid large components (split into smaller)**
5. **Avoid using `watch` when `computed` suffices**

#### ✅ Green Principles (MUST DO)

1. **Use Composition API (Vue 3)**
2. **Use TypeScript with Vue**
3. **Write tests with Vitest + Vue Test Utils**
4. **Use proper key in `v-for`**
5. **Use Pinia for state management**

---

### Angular Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER mutate input properties**
   - ❌ `@Input() data; ngOnInit() { this.data.value = 5 }`
   - ✅ Emit events to parent

2. **NEVER forget to unsubscribe from Observables**
   - ❌ `this.service.getData().subscribe(...)`
   - ✅ Use `takeUntil()` or `async` pipe

3. **NEVER use `any` type**
   - ❌ `data: any`
   - ✅ Define proper interfaces

4. **NEVER bypass Angular sanitization**
   - ❌ `this.sanitizer.bypassSecurityTrustHtml(userInput)`
   - ✅ Sanitize properly

5. **NEVER use `ElementRef.nativeElement` for DOM manipulation**
   - ❌ `this.el.nativeElement.innerHTML = ...`
   - ✅ Use `Renderer2`

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid logic in templates**
2. **Avoid using `ngOnChanges` when `setter` suffices**
3. **Avoid manual subscription (use `async` pipe)**
4. **Avoid large modules (use lazy loading)**
5. **Avoid using `ViewChild` in constructor**

#### ✅ Green Principles (MUST DO)

1. **Use strict TypeScript mode**
2. **Use RxJS operators properly**
3. **Write tests with Jasmine/Karma**
4. **Use OnPush change detection**
5. **Follow Angular style guide**

---

### Svelte Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER mutate props directly**
   - ❌ `export let value; value = newValue`
   - ✅ Use events or two-way binding

2. **NEVER use `@html` with user input**
   - ❌ `{@html userInput}`
   - ✅ Sanitize first

3. **NEVER forget to cleanup subscriptions**
   - ❌ `onMount(() => { store.subscribe(...) })`
   - ✅ `onDestroy(() => { unsubscribe() })`

4. **NEVER use array index as key in `{#each}`**
   - ❌ `{#each items as item, i (i)}`
   - ✅ `{#each items as item (item.id)}`

5. **NEVER access DOM directly without `bind:this`**
   - ❌ `document.querySelector('.my-element')`
   - ✅ `<div bind:this={element}>`

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid large component files**
2. **Avoid using stores for local state**
3. **Avoid deeply nested reactive statements**
4. **Avoid using `$:` for side effects (use `$effect`)**
5. **Avoid mixing Svelte 4 and 5 patterns**

#### ✅ Green Principles (MUST DO)

1. **Use TypeScript with Svelte**
2. **Use SvelteKit for full-stack apps**
3. **Write tests with Vitest + Testing Library**
4. **Use proper keys in `{#each}`**
5. **Use stores for shared state**

---

## 7. Mobile Engineers

### iOS/Swift Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use force unwrapping (`!`) without certainty**
   - ❌ `let value = optional!`
   - ✅ `guard let value = optional else { return }`

2. **NEVER store sensitive data in UserDefaults**
   - ❌ `UserDefaults.standard.set(password, forKey: "pwd")`
   - ✅ Use Keychain

3. **NEVER create retain cycles with closures**
   - ❌ `self.closure = { self.doSomething() }`
   - ✅ `self.closure = { [weak self] in self?.doSomething() }`

4. **NEVER block main thread**
   - ❌ Heavy computation on main queue
   - ✅ Use background queues

5. **NEVER ignore memory warnings**
   - ❌ No `didReceiveMemoryWarning` handling
   - ✅ Clear caches and release resources

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using `as!` force casting**
2. **Avoid massive view controllers**
3. **Avoid using singletons excessively**
4. **Avoid using `NSNotificationCenter` for everything**
5. **Avoid using storyboards for large projects**

#### ✅ Green Principles (MUST DO)

1. **Use SwiftUI for new projects**
2. **Use Combine or async/await**
3. **Write unit tests with XCTest**
4. **Use SwiftLint for code style**
5. **Handle all optionals safely**

---

### Android/Kotlin Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `!!` (force unwrap) without certainty**
   - ❌ `val value = nullable!!`
   - ✅ `val value = nullable ?: return`

2. **NEVER store sensitive data in SharedPreferences**
   - ❌ `prefs.edit().putString("token", jwt).apply()`
   - ✅ Use EncryptedSharedPreferences

3. **NEVER leak context (Activity/Fragment)**
   - ❌ Store Activity reference in static field
   - ✅ Use WeakReference or ViewModel

4. **NEVER perform network calls on main thread**
   - ❌ `URL("...").readText()` on UI thread
   - ✅ Use coroutines or WorkManager

5. **NEVER ignore lifecycle events**
   - ❌ No cleanup in `onDestroy`
   - ✅ Unregister listeners, cancel jobs

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using `findViewById` (use ViewBinding)**
2. **Avoid using AsyncTask (deprecated)**
3. **Avoid using `lateinit` when nullable suffices**
4. **Avoid using `GlobalScope` for coroutines**
5. **Avoid large Activities (use Fragments/Compose)**

#### ✅ Green Principles (MUST DO)

1. **Use Jetpack Compose for UI**
2. **Use Kotlin coroutines for async**
3. **Write tests with JUnit + Espresso**
4. **Use Hilt for dependency injection**
5. **Follow Material Design guidelines**

---

### Flutter/Dart Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `!` (null assertion) without certainty**
   - ❌ `String value = nullable!`
   - ✅ `String? value = nullable`

2. **NEVER store sensitive data in SharedPreferences**
   - ❌ `prefs.setString('token', jwt)`
   - ✅ Use flutter_secure_storage

3. **NEVER build widgets in `build()` method**
   - ❌ `Widget build() { return MyWidget(onTap: () => setState(...)) }`
   - ✅ Extract to methods or separate widgets

4. **NEVER use `setState()` after dispose**
   - ❌ Call `setState` in async callback after widget disposed
   - ✅ Check `mounted` before `setState`

5. **NEVER ignore platform-specific code**
   - ❌ Assume all features work on iOS and Android
   - ✅ Test on both platforms

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid large widget trees (split into smaller)**
2. **Avoid using `GlobalKey` excessively**
3. **Avoid using `setState` for complex state**
4. **Avoid rebuilding entire tree (use `const`)**
5. **Avoid using `ListView` without `builder`**

#### ✅ Green Principles (MUST DO)

1. **Use Riverpod or Bloc for state management**
2. **Use `const` constructors where possible**
3. **Write tests with flutter_test**
4. **Use `flutter analyze` and `dart format`**
5. **Handle null safety properly**

---

### React Native Engineer

#### 🔴 Red Lines (MUST NOT)

1. **NEVER use `eval()` on user input**
   - ❌ `eval(userCode)`
   - ✅ Use proper parsing

2. **NEVER store sensitive data in AsyncStorage**
   - ❌ `AsyncStorage.setItem('token', jwt)`
   - ✅ Use react-native-keychain

3. **NEVER use inline styles for everything**
   - ❌ `<View style={{flex: 1, ...}}>`
   - ✅ Use `StyleSheet.create()`

4. **NEVER ignore platform differences**
   - ❌ Assume iOS and Android behave identically
   - ✅ Use `Platform.select()`

5. **NEVER use `console.log` in production**
   - ❌ Leave debug logs in release builds
   - ✅ Use proper logging library

#### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid using `FlatList` without `keyExtractor`**
2. **Avoid large component files**
3. **Avoid using `ScrollView` for long lists**
4. **Avoid using `Dimensions` directly (use hooks)**
5. **Avoid mixing React Native and native code unnecessarily**

#### ✅ Green Principles (MUST DO)

1. **Use TypeScript**
2. **Use React Navigation for routing**
3. **Write tests with Jest + React Native Testing Library**
4. **Use ESLint + Prettier**
5. **Test on both iOS and Android**

---

## 8. DevOps/SRE Engineer

### 🔴 Red Lines (MUST NOT)

1. **NEVER store secrets in version control**
   - ❌ Commit `.env` files with credentials
   - ✅ Use secret managers (Vault, AWS Secrets Manager)

2. **NEVER run services as root in containers**
   - ❌ `USER root` in Dockerfile
   - ✅ Create non-root user

3. **NEVER deploy without rollback plan**
   - ❌ Push to production without backup
   - ✅ Blue-green or canary deployment

4. **NEVER ignore monitoring and alerting**
   - ❌ No metrics, no alerts
   - ✅ Prometheus + Grafana + PagerDuty

5. **NEVER use `latest` tag in production**
   - ❌ `image: nginx:latest`
   - ✅ `image: nginx:1.25.3`

6. **NEVER expose internal services to internet**
   - ❌ Database port open to 0.0.0.0/0
   - ✅ Use VPC, security groups, firewall

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid manual deployments (automate with CI/CD)**
2. **Avoid single points of failure**
3. **Avoid using SSH for deployments**
4. **Avoid ignoring security patches**
5. **Avoid using default ports (change SSH from 22)**

### ✅ Green Principles (MUST DO)

1. **Implement infrastructure as code (Terraform, Pulumi)**
2. **Use container orchestration (Kubernetes, ECS)**
3. **Set up automated backups with tested restore**
4. **Implement proper logging and tracing**
5. **Use GitOps for deployment (ArgoCD, Flux)**
6. **Conduct regular disaster recovery drills**

---

## 9. Algorithm Engineer

### 🔴 Red Lines (MUST NOT)

1. **NEVER use algorithms with unbounded complexity**
   - ❌ O(n³) algorithm on user-provided input
   - ✅ Optimize or add input size limits

2. **NEVER ignore numerical stability**
   - ❌ `float sum = 0; for (float x : huge_array) sum += x;`
   - ✅ Use Kahan summation or double precision

3. **NEVER use random seeds in production ML models**
   - ❌ `np.random.seed(42)` in production
   - ✅ Use deterministic initialization or save seed

4. **NEVER train on test data**
   - ❌ Use test set for hyperparameter tuning
   - ✅ Separate train/validation/test sets

5. **NEVER ignore data leakage**
   - ❌ Include future data in training
   - ✅ Strict temporal split for time-series

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid premature optimization**
2. **Avoid using deprecated ML libraries**
3. **Avoid overfitting (use regularization)**
4. **Avoid ignoring model interpretability**
5. **Avoid using biased training data**

### ✅ Green Principles (MUST DO)

1. **Document algorithm complexity (Big-O)**
2. **Use proper evaluation metrics**
3. **Version control datasets and models**
4. **Monitor model performance in production**
5. **Test edge cases and boundary conditions**

---

## 10. Product Manager

### 🔴 Red Lines (MUST NOT)

1. **NEVER promise features without engineering estimate**
   - ❌ "We'll ship this next week" (without team input)
   - ✅ Collaborate with engineering on timeline

2. **NEVER ignore user feedback**
   - ❌ Build features without user research
   - ✅ Conduct user interviews and surveys

3. **NEVER change requirements mid-sprint**
   - ❌ Add new scope during active development
   - ✅ Wait for next sprint or negotiate trade-offs

4. **NEVER skip defining success metrics**
   - ❌ Launch feature without KPIs
   - ✅ Define measurable goals upfront

5. **NEVER ignore technical debt**
   - ❌ "Just ship features, ignore bugs"
   - ✅ Allocate time for maintenance

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid feature creep**
2. **Avoid building for edge cases first**
3. **Avoid ignoring competitive analysis**
4. **Avoid skipping beta testing**
5. **Avoid over-engineering MVP**

### ✅ Green Principles (MUST DO)

1. **Write clear user stories with acceptance criteria**
2. **Prioritize ruthlessly (use RICE/MoSCoW)**
3. **Communicate roadmap transparently**
4. **Validate assumptions with data**
5. **Celebrate team wins**

---

## 11. Technical Writer

### 🔴 Red Lines (MUST NOT)

1. **NEVER publish outdated documentation**
   - ❌ Docs reference deprecated APIs
   - ✅ Update docs with every release

2. **NEVER use jargon without explanation**
   - ❌ "Use the XYZ paradigm for ABC"
   - ✅ Define terms or link to glossary

3. **NEVER skip code examples**
   - ❌ "Call the API to get data"
   - ✅ Provide working code snippet

4. **NEVER ignore accessibility in docs**
   - ❌ Images without alt text
   - ✅ Proper headings, alt text, screen reader support

5. **NEVER write docs without testing**
   - ❌ Publish untested instructions
   - ✅ Follow your own docs to verify

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid passive voice**
2. **Avoid walls of text (use headings, lists)**
3. **Avoid assuming prior knowledge**
4. **Avoid using "simply" or "just"**
5. **Avoid outdated screenshots**

### ✅ Green Principles (MUST DO)

1. **Write for your audience (beginner/expert)**
2. **Use consistent terminology**
3. **Provide troubleshooting sections**
4. **Include diagrams and visuals**
5. **Version docs with code releases**

---

## 12. Code Reviewer

### 🔴 Red Lines (MUST NOT)

1. **NEVER approve code without reading it**
   - ❌ "LGTM" without review
   - ✅ Thorough review of logic and tests

2. **NEVER approve code with security vulnerabilities**
   - ❌ SQL injection, XSS, hardcoded secrets
   - ✅ Block PR until fixed

3. **NEVER approve code without tests**
   - ❌ New feature with 0% test coverage
   - ✅ Require tests for new code

4. **NEVER approve breaking changes without discussion**
   - ❌ Remove public API without deprecation
   - ✅ Discuss with team first

5. **NEVER be rude or dismissive**
   - ❌ "This code is terrible"
   - ✅ "Consider refactoring for readability"

### 🟡 Yellow Lines (SHOULD NOT)

1. **Avoid nitpicking style (use linters)**
2. **Avoid blocking on personal preferences**
3. **Avoid reviewing too many PRs at once**
4. **Avoid ignoring CI failures**
5. **Avoid approving without running code locally**

### ✅ Green Principles (MUST DO)

1. **Review within 24 hours**
2. **Provide constructive feedback**
3. **Ask questions to understand intent**
4. **Suggest improvements, don't demand**
5. **Approve when code meets standards**

---

## 🎯 How AI Agents Should Use This Document

### For Code Generation
1. **Before writing code**: Check relevant role's red lines
2. **During implementation**: Follow green principles
3. **Before submitting**: Verify no yellow line violations

### For Code Review
1. **Scan for red line violations** → Block immediately
2. **Check for yellow line issues** → Request changes
3. **Verify green principles** → Approve if met

### For Architecture Decisions
1. **Consult Architect section** for system design
2. **Cross-reference Security** for threat modeling
3. **Check DBA section** for data architecture

### For Debugging
1. **Review relevant language section** for common pitfalls
2. **Check Test Engineer section** for testing gaps
3. **Consult DevOps section** for deployment issues

---

## 📚 References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [Google Engineering Practices](https://google.github.io/eng-practices/)
- [Microsoft Secure Development Lifecycle](https://www.microsoft.com/en-us/securityengineering/sdl)
- [The Twelve-Factor App](https://12factor.net/)

---

## 📝 Contributing

This document is open-source and community-driven. To contribute:

1. Fork the repository
2. Add/update guidelines with clear examples
3. Submit PR with rationale
4. Maintainers review and merge

**Criteria for new guidelines**:
- Must be actionable (clear do/don't)
- Must include examples
- Must cite authoritative source if applicable
- Must be language/framework-agnostic where possible

---

**Version**: 1.0.0  
**Last Updated**: 2026-02-25  
**License**: MIT  
**Maintainers**: Open-source community

---

## 🤖 AI Agent Compliance Checklist

Before submitting any code, AI agents must verify:

- [ ] No red line violations in relevant role sections
- [ ] Yellow line violations documented and justified
- [ ] All green principles followed
- [ ] Tests written and passing
- [ ] Documentation updated
- [ ] Security review completed
- [ ] Performance impact assessed
- [ ] Accessibility considered (if UI)
- [ ] Error handling implemented
- [ ] Logging and monitoring added

**If any red line is violated, code MUST be rejected.**

---

*This document is a living standard. Feedback and contributions welcome.*
