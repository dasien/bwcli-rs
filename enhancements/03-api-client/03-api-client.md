---
slug: api-client
status: NEW
created: 2024-12-02
author: Migration Team
priority: high
---

# Enhancement: CLI Rust Migration - API Client

## Overview
**Goal:** Implement the HTTP API client for communicating with Bitwarden servers, including authentication, error handling, and all necessary endpoints.

**User Story:**
As a developer, I want a robust HTTP client that handles all Bitwarden API communication so that command implementations can focus on business logic without managing HTTP details.

## Context & Background
**Current State:**
- TypeScript CLI uses NodeApiService extending base ApiService
- Supports proxy configuration via environment variables
- Custom User-Agent header
- Token refresh handling
- Configurable server URLs for self-hosted instances
- This is enhancement 3 of 8, depends on enhancements 1 and 2

**Technical Context:**
- Rust project at `bwcli-rs/`
- Will use reqwest with tokio async runtime
- Must integrate with enhancement 2 storage for tokens/config
- Need JSON request/response handling
- TLS support required
- Proxy support from environment variables

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for tokens and URLs)
- reqwest with json and rustls-tls features
- tokio runtime
- serde for JSON handling

## Requirements

### Functional Requirements
1. HTTP client initialization with custom configuration
2. Support GET, POST, PUT, DELETE methods
3. Automatic JSON serialization/deserialization
4. Bearer token authentication header injection
5. Custom headers: User-Agent with version, device type
6. Token refresh flow when access token expires
7. Environment URL resolution (base, API, identity, web vault, icons, notifications)
8. Proxy support from HTTP_PROXY/HTTPS_PROXY environment variables
9. Timeout configuration
10. Error mapping from HTTP status to typed errors

### Non-Functional Requirements
- **Performance:** Connection pooling, typical requests <2s
- **Memory:** Efficient JSON handling, streaming where appropriate
- **Reliability:** Automatic token refresh, clear error types
- **Compatibility:** Work with official and self-hosted Bitwarden servers

### Must Have (MVP)
- [ ] HTTP client wrapper using reqwest
- [ ] Methods for GET, POST, PUT, DELETE
- [ ] JSON request/response handling
- [ ] Bearer token authentication
- [ ] Custom User-Agent header
- [ ] Environment URL resolution
- [ ] Error types for common HTTP errors
- [ ] Proxy support
- [ ] TLS with rustls
- [ ] Connection reuse/pooling

### Should Have (if time permits)
- [ ] Retry logic with exponential backoff
- [ ] Request/response logging (debug mode)
- [ ] Rate limit handling
- [ ] Custom certificate validation for self-hosted
- [ ] Request middleware pattern
- [ ] Connection pool tuning

### Won't Have (out of scope)
- WebSocket support (reason: not needed for CLI)
- HTTP/3 support (reason: not required)
- All API endpoints (reason: implement as needed by commands)

## Open Questions

1. Should token refresh be automatic or manual?
2. What's the appropriate default timeout for various operations?
3. How to handle self-signed certificates for self-hosted servers?
4. Should we implement circuit breaker for failing servers?
5. What level of request/response logging is appropriate?
6. How should we handle API rate limiting?

## Constraints & Limitations
**Technical Constraints:**
- Must work with Bitwarden API specifications
- Must support TLS 1.2+ minimum
- Must validate server certificates by default
- Should reuse connections for performance
- Must work with proxy servers

**Business/Timeline Constraints:**
- Blocking enhancement 4 (authentication commands)
- Blocking enhancement 5 (vault operations)
- Critical path item

## Success Criteria
**Definition of Done:**
- [ ] Client successfully communicates with Bitwarden API
- [ ] Authentication headers properly set
- [ ] Token refresh works automatically
- [ ] Error responses correctly mapped
- [ ] Timeouts work as configured
- [ ] Proxy support functional
- [ ] TLS validation works
- [ ] Unit and integration tests pass
- [ ] Documentation covers all public methods

**Acceptance Tests:**
1. Given valid credentials, when making authenticated request, then Bearer token is included
2. Given expired access token, when making request, then token refresh occurs automatically
3. Given API error 401, when request fails, then AuthenticationError is returned
4. Given network timeout, when request exceeds limit, then TimeoutError is returned
5. Given successful response, when JSON body present, then deserialized correctly
6. Given proxy environment variable, when making request, then uses proxy
7. Given self-hosted URL, when configuring, then all endpoints resolve correctly
8. Given rate limit response, when detected, then appropriate error returned

## Security & Safety Considerations
- Validate TLS certificates by default
- Don't log request/response bodies containing secrets
- Sanitize URLs in logs (hide query params with tokens)
- Use secure default cipher suites
- Validate redirect targets
- Limit response body size to prevent DoS
- Clear sensitive data from memory after use
- Support certificate pinning for enhanced security

## UI/UX Considerations (if applicable)
- Clear error messages for network failures
- Helpful messages for common issues (proxy, firewall, DNS)
- Indicate when retrying requests (if implemented)
- Progress indication for long-running requests (optional)

## Testing Strategy
**Unit Tests:**
- Test request building
- Test header injection
- Test error mapping
- Test URL resolution
- Test JSON serialization/deserialization
- Mock HTTP responses

**Integration Tests:**
- Test against real API endpoints (with test account)
- Test token refresh flow
- Test timeout scenarios
- Test TLS validation
- Test proxy configuration
- Test connection pooling

**Manual Test Scenarios:**
1. Test with official Bitwarden server
2. Test with self-hosted server
3. Test with HTTP proxy configured
4. Test with poor network conditions
5. Compare behavior with TypeScript CLI

## References & Research
- apps/cli/src/platform/services/node-api.service.ts
- libs/common/src/services/api.service.ts
- libs/common/src/models/request/*.ts (request types)
- libs/common/src/models/response/*.ts (response types)
- apps/cli/src/platform/services/default-environment.service.ts
- reqwest documentation: https://docs.rs/reqwest/
- Bitwarden API documentation

## Notes for PM Subagent
- Confirm exact API endpoints needed for MVP
- Verify retry and timeout requirements
- Flag if breaking changes from TypeScript version
- Ensure error messages are user-friendly
- Consider offline operation requirements

## Notes for Architect Subagent
- Design abstraction to hide reqwest details
- Use trait-based approach for testability (MockClient)
- Consider middleware pattern for cross-cutting concerns
- Separate HTTP layer from API client layer
- Plan error hierarchy for different failure types
- Design for async throughout the stack
- Plan for request/response interceptors

## Notes for Implementer Subagent
- Use reqwest::Client as singleton (thread-safe)
- Implement builder pattern for request configuration
- Use reqwest::ClientBuilder for client setup
- Use thiserror for error types
- Implement Debug trait carefully (sanitize sensitive data)
- Use reqwest::Url for URL building and validation
- Add #[serde(rename_all = "camelCase")] for most types
- Some fields use PascalCase, use explicit #[serde(rename)]
- Store client in application state for reuse

## Notes for Testing Subagent
- Use wiremock or mockito for HTTP mocking
- Test with various HTTP status codes
- Test authentication header presence
- Test token refresh flow thoroughly
- Test connection reuse
- Test TLS certificate validation
- Test error handling exhaustively
- Create integration tests with test server
- Test with malformed JSON responses
- Test proxy configuration