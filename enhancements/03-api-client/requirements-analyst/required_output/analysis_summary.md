---
enhancement: 03-api-client
agent: requirements-analyst
task_id: task_1764795876_56521
timestamp: 2025-12-03T15:04:00-08:00
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis: CLI Rust Migration - API Client

## Executive Summary

This enhancement implements the HTTP API client layer for communicating with Bitwarden servers, providing a robust, async-first HTTP abstraction that handles authentication, error mapping, token refresh, proxy support, and all necessary infrastructure for API communication. This is a **critical path** enhancement that directly blocks authentication commands (enhancement 4) and vault operations (enhancement 5).

**Key Insight**: This is pure infrastructure - the API client provides HTTP transport and authentication plumbing, not business logic. Success means creating a reliable, testable HTTP layer that hides complexity from command implementations while maintaining compatibility with Bitwarden's API specifications.

**Dependencies**: Requires completion of enhancements 1 (project-bootstrap) and 2 (storage-layer) for configuration and token storage.

## Business Requirements

### Primary Goal
Create a production-ready HTTP API client that:
- Handles all communication with Bitwarden servers (official and self-hosted)
- Automatically manages authentication tokens and refresh flows
- Provides clear error types that command implementations can handle
- Supports proxy configurations common in enterprise environments
- Maintains security best practices for credential handling

### User Stories

#### Story 1: Authenticated API Requests
**As a** CLI user with valid credentials,
**I want** the CLI to automatically include my authentication token in API requests,
**So that** I can access my vault without manually managing tokens.

**Acceptance Criteria**:
- [ ] Client automatically includes Bearer token in Authorization header
- [ ] Token is retrieved from storage layer (enhancement 2)
- [ ] Requests fail with clear error if token is missing
- [ ] Token is never logged or printed in error messages
- [ ] Multiple concurrent requests reuse the same token

**Complexity**: Medium (3 points)

#### Story 2: Automatic Token Refresh
**As a** CLI user whose access token has expired,
**I want** the CLI to automatically refresh my token using my refresh token,
**So that** I don't have to re-authenticate every time my token expires.

**Acceptance Criteria**:
- [ ] Client detects 401 Unauthorized responses indicating expired token
- [ ] Client attempts token refresh using stored refresh token
- [ ] On successful refresh, new tokens are saved to storage
- [ ] Original request is retried with new access token
- [ ] User sees seamless operation without interruption
- [ ] Token refresh failure results in clear "please login again" message
- [ ] Concurrent requests wait for single token refresh operation

**Complexity**: High (5 points)

#### Story 3: Enterprise Proxy Support
**As a** CLI user behind a corporate proxy,
**I want** the CLI to respect my HTTP_PROXY/HTTPS_PROXY environment variables,
**So that** I can use the CLI from my corporate network.

**Acceptance Criteria**:
- [ ] Client reads HTTP_PROXY environment variable
- [ ] Client reads HTTPS_PROXY environment variable
- [ ] Proxy authentication (username:password in URL) is supported
- [ ] Connection errors provide clear proxy troubleshooting hints
- [ ] Client respects NO_PROXY environment variable for excluded hosts
- [ ] Self-hosted servers work through proxies

**Complexity**: Medium (3 points)

#### Story 4: Self-Hosted Server Support
**As a** user of a self-hosted Bitwarden server,
**I want** to configure custom server URLs,
**So that** I can use the CLI with my own Bitwarden instance.

**Acceptance Criteria**:
- [ ] Client resolves all environment URLs (api, identity, web vault, icons, notifications)
- [ ] Custom base URL can be configured via storage/config
- [ ] Each service URL can be overridden independently
- [ ] URL validation prevents invalid configurations
- [ ] Clear error messages for unreachable servers
- [ ] TLS certificate validation works with custom domains

**Complexity**: Medium (3 points)

#### Story 5: Clear Error Messages
**As a** CLI user experiencing API errors,
**I want** to receive clear, actionable error messages,
**So that** I can understand what went wrong and how to fix it.

**Acceptance Criteria**:
- [ ] Network errors include helpful troubleshooting (check DNS, proxy, firewall)
- [ ] Timeout errors suggest increasing timeout or checking network
- [ ] Authentication errors suggest running `bw login`
- [ ] Rate limit errors include retry-after information
- [ ] Server errors include status code and API error message
- [ ] Connection refused suggests checking server URL configuration
- [ ] TLS errors suggest certificate or proxy issues

**Complexity**: Medium (3 points)

## Functional Requirements

### FR-1: HTTP Client Infrastructure
- **FR-1.1**: Use `reqwest` v0.12+ with rustls-tls feature for TLS
- **FR-1.2**: Client is async using tokio runtime
- **FR-1.3**: Client singleton with connection pooling (reuse across requests)
- **FR-1.4**: Configurable timeouts (connect, read, total request)
- **FR-1.5**: Custom User-Agent header format: `Bitwarden_CLI/{version} (Rust)`
- **FR-1.6**: Support GET, POST, PUT, DELETE HTTP methods
- **FR-1.7**: Automatic JSON serialization for request bodies
- **FR-1.8**: Automatic JSON deserialization for response bodies

### FR-2: Authentication & Token Management
- **FR-2.1**: Inject Bearer token in Authorization header for authenticated requests
- **FR-2.2**: Retrieve tokens from storage layer (enhancement 2)
- **FR-2.3**: Detect expired access tokens (HTTP 401 responses)
- **FR-2.4**: Automatic token refresh using refresh token
- **FR-2.5**: Save refreshed tokens back to storage
- **FR-2.6**: Retry original request after successful token refresh
- **FR-2.7**: Fail with authentication error if refresh token is invalid/expired
- **FR-2.8**: Prevent concurrent token refresh (use mutex/lock)

### FR-3: Environment URL Resolution
- **FR-3.1**: Support base URL configuration (default: `https://vault.bitwarden.com`)
- **FR-3.2**: Resolve API URL: `{base}/api`
- **FR-3.3**: Resolve identity URL: `{base}/identity`
- **FR-3.4**: Resolve web vault URL: `{base}`
- **FR-3.5**: Resolve icons URL: `{base}/icons`
- **FR-3.6**: Resolve notifications URL: `{base}/notifications`
- **FR-3.7**: Allow independent override of each service URL
- **FR-3.8**: Validate URLs are well-formed and use HTTPS (or HTTP for localhost)

### FR-4: Proxy Support
- **FR-4.1**: Read HTTP_PROXY environment variable
- **FR-4.2**: Read HTTPS_PROXY environment variable
- **FR-4.3**: Support proxy authentication (username:password in URL)
- **FR-4.4**: Respect NO_PROXY environment variable
- **FR-4.5**: Configure proxy at client initialization time

### FR-5: Error Handling
- **FR-5.1**: Define error types for all failure scenarios:
  - Network errors (DNS, connection refused, timeout)
  - HTTP errors (4xx, 5xx with status code and message)
  - Authentication errors (401, 403)
  - Rate limit errors (429 with retry-after)
  - Serialization errors (JSON parsing failures)
  - TLS errors (certificate validation failures)
- **FR-5.2**: Include context in errors (URL, method, status code)
- **FR-5.3**: Map Bitwarden API error responses to typed errors
- **FR-5.4**: Provide user-friendly error messages with troubleshooting hints

### FR-6: Security & Safety
- **FR-6.1**: Validate TLS certificates by default
- **FR-6.2**: Use secure default cipher suites (via rustls)
- **FR-6.3**: Never log request/response bodies containing secrets
- **FR-6.4**: Sanitize URLs in logs (hide tokens in query params)
- **FR-6.5**: Limit response body size to prevent DoS
- **FR-6.6**: Validate redirect targets to prevent open redirects
- **FR-6.7**: Clear sensitive data from memory after use (use secrecy/zeroize)

### FR-7: Request/Response Processing
- **FR-7.1**: Build requests with method, URL, headers, body
- **FR-7.2**: Set Content-Type: application/json for JSON requests
- **FR-7.3**: Set Accept: application/json for JSON responses
- **FR-7.4**: Parse successful responses (2xx) into expected types
- **FR-7.5**: Parse error responses into error types
- **FR-7.6**: Handle empty response bodies (204 No Content)

## Non-Functional Requirements

### NFR-1: Performance
- **NFR-1.1**: Typical API requests complete in <2 seconds
- **NFR-1.2**: Connection pooling reuses TCP connections
- **NFR-1.3**: Minimal memory overhead for JSON processing
- **NFR-1.4**: Concurrent requests supported without blocking
- **NFR-1.5**: Token refresh adds <500ms overhead to affected requests

### NFR-2: Reliability
- **NFR-2.1**: Automatic token refresh prevents authentication failures
- **NFR-2.2**: Clear error types enable proper error handling
- **NFR-2.3**: Transient network failures are detectable vs permanent failures
- **NFR-2.4**: No data loss during token refresh operations
- **NFR-2.5**: Thread-safe for concurrent use

### NFR-3: Security
- **NFR-3.1**: TLS 1.2+ required for all connections
- **NFR-3.2**: Certificate validation enabled by default
- **NFR-3.3**: Secrets never logged or printed
- **NFR-3.4**: Defense against open redirects
- **NFR-3.5**: Protection against response body DoS

### NFR-4: Compatibility
- **NFR-4.1**: Works with official Bitwarden cloud (vault.bitwarden.com)
- **NFR-4.2**: Works with self-hosted Bitwarden servers
- **NFR-4.3**: Compatible with Bitwarden API v1 specifications
- **NFR-4.4**: Proxy support matches enterprise requirements
- **NFR-4.5**: Behavior matches TypeScript CLI API client

### NFR-5: Maintainability
- **NFR-5.1**: Error types are comprehensive and well-documented
- **NFR-5.2**: Client abstraction hides reqwest implementation details
- **NFR-5.3**: Testable via trait abstraction (mock implementations)
- **NFR-5.4**: Clear separation between HTTP layer and API client layer
- **NFR-5.5**: Logging supports debugging without exposing secrets

## Risks & Challenges

### High Priority Risks

#### Risk 1: Token Refresh Race Conditions
**Description**: Multiple concurrent requests with expired token could trigger multiple simultaneous token refresh attempts, potentially invalidating tokens or causing confusion.

**Impact**: High - Could cause authentication failures, token invalidation, or poor user experience.

**Mitigation**:
- Use mutex/lock to ensure only one token refresh happens at a time
- Queue concurrent requests to wait for in-progress refresh
- Test concurrent scenarios thoroughly

**Owner**: Architect + Implementer

#### Risk 2: Proxy Configuration Complexity
**Description**: Proxy configurations vary widely (HTTP/HTTPS, authentication, NO_PROXY patterns), and troubleshooting proxy issues is difficult.

**Impact**: Medium - Users behind proxies may be unable to use CLI without extensive troubleshooting.

**Mitigation**:
- Provide detailed error messages for proxy-related failures
- Document proxy configuration clearly
- Add debug logging for proxy configuration (sanitized)
- Test with various proxy configurations

**Owner**: Implementer + Tester

#### Risk 3: API Error Response Format Changes
**Description**: Bitwarden API error responses could have format variations that break error parsing.

**Impact**: Medium - Poor error messages or parsing failures instead of graceful handling.

**Mitigation**:
- Use flexible JSON parsing (serde defaults)
- Fall back to generic error if parsing fails
- Extract whatever error context is available
- Test with various API error responses

**Owner**: Implementer

### Medium Priority Risks

#### Risk 4: TLS Certificate Validation Edge Cases
**Description**: Self-signed certificates, expired certificates, certificate pinning, and custom CA certificates all require different handling.

**Impact**: Medium - Self-hosted users may have certificate issues blocking CLI usage.

**Mitigation**:
- Provide clear TLS error messages with troubleshooting steps
- Document certificate requirements
- Consider allowing certificate validation override (with warnings)
- Future enhancement could add custom CA certificate support

**Owner**: Architect (design decision needed)

#### Risk 5: Timeout Configuration Trade-offs
**Description**: Too short = failures on slow networks, too long = poor user experience waiting for failures.

**Impact**: Low - Configurable timeouts mitigate this, but defaults matter.

**Mitigation**:
- Research TypeScript CLI timeout values
- Use sensible defaults (30s connect, 60s read)
- Allow configuration override
- Document timeout behavior

**Owner**: Implementer

## Open Questions & Decisions Needed

### Critical Questions (Block Architecture)

1. **Token Refresh Strategy**
   - **Question**: Should token refresh be fully automatic (invisible to caller), or should some requests opt out?
   - **Context**: Automatic refresh is convenient but adds complexity. What if refresh fails mid-operation?
   - **Options**:
     - A) Fully automatic for all authenticated requests (recommended)
     - B) Caller can disable refresh for specific requests
     - C) Return special error type that caller must handle
   - **Recommendation**: Option A - matches TypeScript CLI behavior, best UX
   - **Decision Maker**: Architect

2. **Error Type Granularity**
   - **Question**: How detailed should error types be? One error per HTTP status? Categories?
   - **Context**: Too many error types = burden on callers, too few = lost context
   - **Options**:
     - A) Error per common scenario (Network, Auth, RateLimit, Server, Client, Timeout)
     - B) Error per HTTP status code (400, 401, 403, 404, 429, 500, 502, 503)
     - C) Single HTTP error with status code field
   - **Recommendation**: Option A with status code field - balances usability and detail
   - **Decision Maker**: Architect

3. **Client Abstraction Level**
   - **Question**: Should we expose reqwest types directly or wrap everything?
   - **Context**: Wrapping provides flexibility but adds code, exposing reqwest is simpler
   - **Options**:
     - A) Trait-based abstraction hiding all reqwest details (max flexibility)
     - B) Thin wrapper exposing some reqwest types (method, header types)
     - C) Direct reqwest usage (minimal abstraction)
   - **Recommendation**: Option A - enables testing, future changes (swap HTTP client)
   - **Decision Maker**: Architect

### Important Questions (Inform Implementation)

4. **Request/Response Logging**
   - **Question**: What level of request/response logging should we provide?
   - **Context**: Helpful for debugging but risks exposing secrets
   - **Options**:
     - A) No logging (safest)
     - B) Headers only (no bodies)
     - C) Full logging behind debug flag with sanitization
   - **Recommendation**: Option C - critical for troubleshooting, must sanitize carefully
   - **Decision Maker**: Implementer

5. **Retry Logic**
   - **Question**: Should we implement retry logic for transient failures?
   - **Context**: Listed as "should have" - nice to have but adds complexity
   - **Options**:
     - A) No retries in MVP (caller handles)
     - B) Retry transient errors (timeouts, 502/503) with exponential backoff
     - C) Retry all failures except auth errors
   - **Recommendation**: Option A for MVP - can add later without breaking changes
   - **Decision Maker**: Architect

6. **Rate Limit Handling**
   - **Question**: Should client automatically handle 429 rate limit responses?
   - **Context**: Could auto-wait and retry, or return error immediately
   - **Options**:
     - A) Return error immediately with retry-after info
     - B) Automatically wait and retry (up to limit)
   - **Recommendation**: Option A for MVP - simpler, gives caller control
   - **Decision Maker**: Architect

7. **Certificate Validation Overrides**
   - **Question**: Should we allow disabling certificate validation for self-hosted?
   - **Context**: Some self-hosted users have self-signed certificates
   - **Options**:
     - A) No override - enforce certificate validation always
     - B) Allow override via flag (with prominent warning)
     - C) Support custom CA certificates
   - **Recommendation**: Defer to architect - security vs usability trade-off
   - **Decision Maker**: Architect (security consideration)

8. **Connection Pool Configuration**
   - **Question**: Should connection pool settings be configurable?
   - **Context**: reqwest has defaults, but some users may need tuning
   - **Options**:
     - A) Use reqwest defaults (10 connections per host)
     - B) Make configurable via environment variables
     - C) Hard-code optimal values
   - **Recommendation**: Option A - defaults are sensible, can expose later if needed
   - **Decision Maker**: Implementer

## Constraints & Assumptions

### Technical Constraints
- **C-1**: Must use async Rust with tokio runtime (project standard)
- **C-2**: Must use reqwest with rustls for TLS (not native-tls)
- **C-3**: Must maintain compatibility with Bitwarden API specifications
- **C-4**: Cannot modify Bitwarden server behavior
- **C-5**: Must integrate with enhancement 2 storage layer for tokens/config
- **C-6**: JSON serialization uses serde (project standard)
- **C-7**: Error types use thiserror (project standard)

### Business Constraints
- **C-8**: Blocks enhancement 4 (authentication) - cannot proceed without this
- **C-9**: Blocks enhancement 5 (vault operations) - critical path
- **C-10**: Must complete before auth commands can be tested end-to-end
- **C-11**: Self-hosted server support is non-negotiable (existing feature)

### Assumptions
- **A-1**: Bitwarden API v1 specifications are stable and documented
- **A-2**: TypeScript CLI behavior is reference implementation
- **A-3**: reqwest 0.12+ has required features and stability
- **A-4**: Storage layer (enhancement 2) is complete and tested
- **A-5**: Token storage includes both access and refresh tokens
- **A-6**: Token refresh endpoint follows OAuth2 refresh token flow
- **A-7**: Environment URLs follow documented Bitwarden patterns
- **A-8**: Proxy environment variables follow standard conventions

## Success Criteria

### Definition of Done
- [ ] Client successfully makes GET/POST/PUT/DELETE requests to Bitwarden API
- [ ] Bearer token automatically included in authenticated requests
- [ ] Token refresh works automatically on 401 responses
- [ ] Refreshed tokens saved to storage layer
- [ ] HTTP errors correctly mapped to typed error enums
- [ ] Proxy support functional with HTTP_PROXY/HTTPS_PROXY
- [ ] Custom User-Agent header included in all requests
- [ ] Environment URL resolution works for all service types
- [ ] TLS certificate validation enabled and functional
- [ ] Connection pooling demonstrates reuse across requests
- [ ] Timeout configuration works as expected
- [ ] All unit tests pass (>80% code coverage)
- [ ] Integration tests pass with real/mock API
- [ ] Documentation covers all public APIs
- [ ] Error messages are clear and actionable

### Acceptance Tests

#### Test Suite 1: Basic HTTP Operations
1. **Given** valid API endpoint, **when** making GET request, **then** response is deserialized correctly
2. **Given** JSON payload, **when** making POST request, **then** request serializes and sends correctly
3. **Given** resource ID, **when** making PUT request, **then** resource updates successfully
4. **Given** resource ID, **when** making DELETE request, **then** resource deletes successfully
5. **Given** request timeout, **when** request exceeds timeout, **then** TimeoutError returned with helpful message

#### Test Suite 2: Authentication & Token Management
1. **Given** valid access token in storage, **when** making authenticated request, **then** Bearer token included in Authorization header
2. **Given** missing access token, **when** making authenticated request, **then** AuthenticationError returned with "please login" message
3. **Given** expired access token, **when** making authenticated request, **then** 401 triggers automatic token refresh
4. **Given** successful token refresh, **when** refresh completes, **then** new tokens saved to storage
5. **Given** successful token refresh, **when** refresh completes, **then** original request retried with new token
6. **Given** invalid refresh token, **when** refresh fails, **then** AuthenticationError returned with "please login again"
7. **Given** concurrent requests with expired token, **when** multiple requests trigger refresh, **then** only one refresh occurs (no race)

#### Test Suite 3: Environment URL Resolution
1. **Given** default configuration, **when** resolving API URL, **then** returns `https://vault.bitwarden.com/api`
2. **Given** custom base URL, **when** resolving API URL, **then** returns `{custom}/api`
3. **Given** custom base URL, **when** resolving identity URL, **then** returns `{custom}/identity`
4. **Given** independent service URLs, **when** overridden, **then** uses override values
5. **Given** invalid URL format, **when** validating configuration, **then** returns clear error
6. **Given** HTTP URL for localhost, **when** validating, **then** allows HTTP exception
7. **Given** HTTP URL for remote host, **when** validating, **then** rejects with HTTPS requirement

#### Test Suite 4: Proxy Support
1. **Given** HTTP_PROXY set, **when** making request, **then** uses proxy server
2. **Given** HTTPS_PROXY set, **when** making HTTPS request, **then** uses proxy server
3. **Given** proxy with authentication, **when** making request, **then** authenticates to proxy
4. **Given** NO_PROXY with matching host, **when** making request, **then** bypasses proxy
5. **Given** unreachable proxy, **when** connection fails, **then** error message suggests checking proxy configuration

#### Test Suite 5: Error Handling
1. **Given** DNS resolution failure, **when** request fails, **then** NetworkError with DNS troubleshooting hint
2. **Given** connection refused, **when** request fails, **then** NetworkError suggesting checking URL and server status
3. **Given** API returns 404, **when** parsing response, **then** NotFoundError with resource context
4. **Given** API returns 429, **when** parsing response, **then** RateLimitError with retry-after information
5. **Given** API returns 500, **when** parsing response, **then** ServerError with status code and message
6. **Given** API returns 503, **when** parsing response, **then** ServerError suggesting temporary issue
7. **Given** malformed JSON response, **when** deserializing, **then** SerializationError with context
8. **Given** TLS certificate invalid, **when** connection fails, **then** TlsError with certificate issue details

#### Test Suite 6: Security & Safety
1. **Given** authenticated request, **when** logging request, **then** Authorization header value is sanitized
2. **Given** request with token in URL, **when** logging URL, **then** token query param is sanitized
3. **Given** error with sensitive data, **when** displaying error, **then** sensitive fields redacted
4. **Given** redirect response, **when** following redirect, **then** validates redirect target is same origin or whitelisted
5. **Given** response body >10MB, **when** reading response, **then** stops reading and returns error
6. **Given** HTTPS connection, **when** TLS handshake, **then** validates certificate chain

#### Test Suite 7: Performance & Reliability
1. **Given** multiple sequential requests to same host, **when** checking connections, **then** TCP connection is reused
2. **Given** 100 concurrent requests, **when** all executing, **then** all complete successfully without blocking
3. **Given** token refresh during concurrent requests, **when** refresh completes, **then** all waiting requests use new token

## Project Phases

### Phase 1: Core HTTP Infrastructure (Foundation)
**Goal**: Basic HTTP client wrapper with configuration

**Deliverables**:
- HTTP client initialization using reqwest
- GET/POST/PUT/DELETE method support
- JSON serialization/deserialization
- Custom User-Agent header
- Basic error types
- Client configuration structure

**Success Criteria**: Can make simple HTTP requests and receive responses

**Estimated Complexity**: Medium

**Dependencies**: Enhancement 1 (project-bootstrap)

---

### Phase 2: Authentication & Token Management (Critical Path)
**Goal**: Automatic token injection and refresh

**Deliverables**:
- Bearer token injection from storage
- Token refresh detection (401 responses)
- Token refresh endpoint implementation
- Save refreshed tokens to storage
- Retry original request after refresh
- Race condition prevention (mutex)

**Success Criteria**: Authenticated requests work with automatic token refresh

**Estimated Complexity**: High (complex concurrency)

**Dependencies**: Enhancement 2 (storage-layer), Phase 1

---

### Phase 3: Environment & Proxy Support (Compatibility)
**Goal**: Self-hosted and enterprise environment support

**Deliverables**:
- Environment URL resolution
- Base URL configuration
- Independent service URL overrides
- HTTP_PROXY/HTTPS_PROXY support
- NO_PROXY support
- Proxy authentication

**Success Criteria**: Works with self-hosted servers and through corporate proxies

**Estimated Complexity**: Medium

**Dependencies**: Phase 1

---

### Phase 4: Error Handling & Safety (Production Readiness)
**Goal**: Comprehensive error handling and security

**Deliverables**:
- Complete error type hierarchy
- Error context and troubleshooting hints
- TLS certificate validation
- Request/response sanitization
- Logging with secret redaction
- Response size limits
- Redirect validation

**Success Criteria**: All error scenarios handled gracefully with clear messages

**Estimated Complexity**: Medium

**Dependencies**: Phases 1-3

---

### Phase 5: Testing & Documentation (Quality)
**Goal**: Comprehensive test coverage and documentation

**Deliverables**:
- Unit tests (80%+ coverage)
- Integration tests with mock server
- Security tests
- Performance tests
- API documentation
- Troubleshooting guide
- Examples for command implementations

**Success Criteria**: All tests pass, documentation complete

**Estimated Complexity**: Medium

**Dependencies**: Phases 1-4

## Integration Points

### Upstream Dependencies (Required Before This Enhancement)
- **Enhancement 1 (project-bootstrap)**: Provides workspace, dependencies, async runtime
- **Enhancement 2 (storage-layer)**: Provides token storage, configuration storage, URL configuration

### Downstream Dependencies (Blocked By This Enhancement)
- **Enhancement 4 (auth-commands)**: Login, logout, unlock commands need API client
- **Enhancement 5 (vault-read-commands)**: List, get commands need API client
- **Enhancement 6 (vault-write-commands)**: Create, update, delete need API client
- **Enhancement 7 (tool-commands)**: Generate, encode commands may need API for sync checks

### External Integrations
- **Bitwarden API**: Primary integration point - all API v1 endpoints
- **Bitwarden Identity Server**: OAuth2 token endpoints for authentication
- **Bitwarden SDK**: May provide API models (request/response types)

### Internal Module Interactions
- **Storage Module**: Read tokens, URLs, configuration; write refreshed tokens
- **Command Handlers**: All commands making API calls depend on this client
- **Error Handling Module**: API errors propagate to command error responses
- **Logging Module**: Debug logging of requests/responses (sanitized)

## Technical Flags for Architect

### High Priority Architectural Decisions Needed

1. **HTTP Client Trait Design**
   - **Flag**: Need trait abstraction vs concrete reqwest types
   - **Reason**: Affects testability (mock implementations) and future flexibility
   - **Recommendation**: Define `ApiClient` trait with methods for each HTTP operation
   - **Impact**: All command implementations depend on this interface

2. **Token Refresh Concurrency Model**
   - **Flag**: How to prevent token refresh race conditions
   - **Reason**: Multiple concurrent expired requests could trigger multiple refreshes
   - **Recommendation**: Use Arc<Mutex<Option<RefreshFuture>>> to coordinate
   - **Impact**: Complex implementation, critical for correctness

3. **Error Type Hierarchy**
   - **Flag**: Structure and granularity of error types
   - **Reason**: Determines how commands handle different failure scenarios
   - **Recommendation**: Enum with variants (Network, Auth, RateLimit, Server, Client, Timeout) + status code
   - **Impact**: Command error handling, user-facing messages

4. **Request/Response Middleware Pattern**
   - **Flag**: Should we support middleware for cross-cutting concerns?
   - **Reason**: Logging, metrics, retry logic could be middleware
   - **Recommendation**: Start simple, add middleware pattern if needed later
   - **Impact**: Architecture complexity vs flexibility

### Medium Priority Architectural Considerations

5. **Environment URL Structure**
   - **Flag**: How to model and validate environment URLs
   - **Reason**: Multiple service URLs, overrides, validation rules
   - **Recommendation**: `Environment` struct with builder pattern and validation
   - **Impact**: Configuration API, self-hosted support

6. **JSON Serialization Strategy**
   - **Flag**: Generic serialization vs typed request/response structs
   - **Reason**: Trade-off between type safety and boilerplate
   - **Recommendation**: Typed structs for common operations, generic for flexibility
   - **Impact**: API surface area, code generation needs

7. **Timeout Configuration Model**
   - **Flag**: Single timeout vs separate connect/read/total timeouts
   - **Reason**: Different timeout types serve different purposes
   - **Recommendation**: Separate timeouts with sensible defaults (30s connect, 60s read)
   - **Impact**: Client configuration complexity

### Low Priority Architectural Notes

8. **Connection Pool Configuration**
   - **Note**: reqwest defaults (10 connections) likely sufficient for CLI
   - **Defer**: Can expose configuration later if needed

9. **HTTP/2 Support**
   - **Note**: reqwest supports HTTP/2 automatically
   - **Action**: Enable by default, no special handling needed

10. **Client Lifecycle Management**
    - **Note**: Client should be singleton (created once, reused)
    - **Recommendation**: Store in application state, pass as reference

## Notes for Downstream Agents

### For Architect Agent
- Review "Technical Flags for Architect" section above carefully
- All high priority decisions must be made in architecture phase
- Consider reviewing TypeScript NodeApiService and base ApiService for patterns
- Token refresh concurrency is the most complex design challenge
- Error type design affects all downstream command implementations
- Trait design determines testability strategy

### For Implementer Agent
- reqwest::Client is thread-safe (Arc internally) - can be cloned cheaply
- Use reqwest::ClientBuilder for client initialization
- Token storage access requires async - all methods must be async
- Use thiserror for error types with proper Display implementations
- Implement Debug carefully - sanitize all sensitive data
- Use tracing crate for logging, not println
- #[serde(rename_all = "camelCase")] for most Bitwarden API types
- Some Bitwarden fields use PascalCase - check API responses
- Token refresh mutex needs careful testing for deadlocks

### For Tester Agent
- Use wiremock or mockito for HTTP mocking in tests
- Mock token storage layer in tests
- Test all HTTP status codes (200, 204, 400, 401, 403, 404, 429, 500, 502, 503)
- Test token refresh thoroughly including race conditions
- Test with intentionally slow responses for timeout testing
- Test with malformed JSON to verify error handling
- Integration tests should use test Bitwarden account (if available)
- Security tests must verify no secrets in logs/errors
- Test connection pooling with HTTP client inspection

### For Documenter Agent
- API client is internal infrastructure - focus on troubleshooting guide
- Document error types and what triggers them
- Document proxy configuration clearly
- Document self-hosted server URL configuration
- Document timeout behavior and configuration
- Provide examples for command implementations using the client
- Include troubleshooting section for common issues (network, proxy, TLS)

## Validation Criteria

### Requirements Completeness ✅
- [x] All functional requirements identified and documented
- [x] Non-functional requirements specified with measurable targets
- [x] Must have/should have/won't have clearly categorized
- [x] Integration points with other enhancements documented

### Clarity & Unambiguity ✅
- [x] User stories follow standard format with acceptance criteria
- [x] Technical terminology used consistently
- [x] Assumptions documented explicitly
- [x] Constraints identified clearly

### Testability ✅
- [x] Every requirement has associated acceptance test
- [x] Success criteria are measurable
- [x] Test scenarios cover happy path and error cases
- [x] Performance targets specified numerically

### Architecture Readiness ✅
- [x] Technical challenges identified and flagged
- [x] Architectural decision points documented
- [x] Recommendations provided for architect review
- [x] Dependencies and integration points clear

## Appendix A: Reference Materials

### TypeScript CLI Source Files (for migration reference)
- `apps/cli/src/platform/services/node-api.service.ts` - Main API service
- `libs/common/src/services/api.service.ts` - Base API implementation
- `libs/common/src/abstractions/api.service.ts` - API interface definition
- `apps/cli/src/platform/services/default-environment.service.ts` - URL resolution
- `libs/common/src/models/request/*.ts` - API request types
- `libs/common/src/models/response/*.ts` - API response types

### External Documentation
- reqwest documentation: https://docs.rs/reqwest/
- rustls documentation: https://docs.rs/rustls/
- tokio documentation: https://docs.rs/tokio/
- Bitwarden API documentation: https://bitwarden.com/help/api/
- OAuth2 RFC 6749 (token refresh): https://tools.ietf.org/html/rfc6749#section-6
- HTTP status codes: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status

### Related Enhancements
- Enhancement 1: project-bootstrap - Workspace setup
- Enhancement 2: storage-layer - Token/config storage
- Enhancement 4: auth-commands - First consumer of API client
- Enhancement 5: vault-read-commands - Second consumer
- Enhancement 6: vault-write-commands - Third consumer

## Appendix B: Glossary

- **API Client**: HTTP client wrapper providing Bitwarden-specific functionality
- **Bearer Token**: OAuth2 access token sent in Authorization header
- **Token Refresh**: Process of obtaining new access token using refresh token
- **Environment URL**: Service-specific base URL (api, identity, web vault, icons, notifications)
- **Proxy**: Intermediate server for HTTP requests (common in corporate networks)
- **Self-Hosted**: User-operated Bitwarden server instance (not official cloud)
- **Rate Limiting**: Server throttling requests (HTTP 429 response)
- **Connection Pooling**: Reusing TCP connections across multiple requests
- **TLS Certificate Validation**: Verifying server identity via certificate chain
- **reqwest**: Rust HTTP client library
- **rustls**: Rust TLS implementation (alternative to OpenSSL)
- **OAuth2**: Authorization framework used by Bitwarden for authentication

---

## Status Summary

**Status**: `READY_FOR_ARCHITECTURE`

**Rationale**:
- All functional and non-functional requirements extracted and documented
- User stories created with clear acceptance criteria
- Risks identified and mitigation strategies proposed
- Critical architectural decisions flagged for architect review
- Open questions documented for architect to resolve
- Success criteria defined with measurable acceptance tests
- Integration points and dependencies clearly mapped
- Constraints and assumptions documented
- Project phases outlined with logical progression

**Next Steps**:
1. Architect agent reviews analysis and architectural decision points
2. Architect designs API client trait and error type hierarchy
3. Architect resolves token refresh concurrency strategy
4. Architect creates technical specification for implementation
5. Implementer agent follows architectural spec to build client

**Blocking Issues**: None - ready for architecture phase

**Notes**:
- Token refresh concurrency is the most complex technical challenge
- Error type design is critical - affects all downstream commands
- Trait abstraction decision affects testability strategy
- All high priority architectural decisions must be resolved before implementation
