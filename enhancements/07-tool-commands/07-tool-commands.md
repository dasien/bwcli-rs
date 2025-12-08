---
slug: tool-commands
status: NEW
created: 2024-12-02
author: Migration Team
priority: medium
---

# Enhancement: CLI Rust Migration - Tools Commands

## Overview
**Goal:** Implement generate, send, receive, and encode commands for password generation and secure sharing.

**User Story:**
As a CLI user, I want to generate passwords, create Send links, and encode data so that I can manage secure information without storing it in my vault.

## Context & Background
**Current State:**
- TypeScript CLI implements generate (password/passphrase), send (CRUD), receive, and encode commands
- Generate supports customizable password and passphrase generation
- Send supports temporary secure sharing with expiration and access limits
- Receive retrieves Send content
- Encode base64-encodes input
- This is enhancement 7 of 8, depends on enhancements 1-6

**Technical Context:**
- Rust project at `bwcli-rs/`
- Generate uses secure random generation
- Send requires encryption using SDK
- Uses API client for Send operations
- May use Bitwarden SDK generator or implement directly

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for Send encryption keys)
- Enhancement: api-client (for Send API)
- Enhancement: authentication-commands (for authenticated Send)
- Bitwarden SDK for encryption and generation

## Requirements

### Functional Requirements
1. Generate password with customizable options (length, uppercase, lowercase, numbers, special)
2. Generate passphrase with customizable word count and separator
3. Generate passwords with excluded characters
4. Generate with minimum numbers and special characters
5. Send create command for text and file Sends
6. Send list command to show existing Sends
7. Send get command to retrieve Send details
8. Send edit command to modify Send properties
9. Send delete command to remove Send
10. Send remove-password command
11. Receive command to access Send content
12. Encode command for base64 encoding
13. Support Send templates

### Non-Functional Requirements
- **Performance:** Generate <100ms, Send operations <2s
- **Memory:** Efficient handling of Send files
- **Reliability:** Secure random generation, proper encryption
- **Compatibility:** Output matches TypeScript CLI format

### Must Have (MVP)
- [ ] `bw generate` with default options
- [ ] `bw generate --length <n>`
- [ ] `bw generate --passphrase`
- [ ] `bw generate` with custom character options
- [ ] `bw send create` for text
- [ ] `bw send list`
- [ ] `bw send get <id>`
- [ ] `bw send delete <id>`
- [ ] `bw receive <url>`
- [ ] `bw encode`
- [ ] Secure random generation
- [ ] Send encryption using SDK

### Should Have (if time permits)
- [ ] `bw send create` for files
- [ ] `bw send edit <id>`
- [ ] `bw send remove-password <id>`
- [ ] `bw send template`
- [ ] Send with expiration dates
- [ ] Send with access count limits
- [ ] Send with password protection
- [ ] Generate with excluded characters
- [ ] Generate with minimum requirements

### Won't Have (out of scope)
- Custom word lists for passphrase (reason: complexity)
- Send analytics (reason: not in API)
- Batch Send operations (reason: not MVP)

## Open Questions

1. Should we implement generator directly or use SDK?
2. What's the default password length and character set?
3. How to handle large file Sends efficiently?
4. Should Send operations require authentication?
5. What's the maximum Send file size?
6. Should we support Send expiration editing?

## Constraints & Limitations
**Technical Constraints:**
- Must use cryptographically secure random
- Send files limited by API size constraints
- Send encryption must match web vault format
- Must handle Send expiration properly
- Generate must meet security best practices

**Business/Timeline Constraints:**
- Can be partially implemented (generate first, then Send)
- Not blocking other enhancements
- Nice to have for feature parity

## Success Criteria
**Definition of Done:**
- [ ] `bw generate` creates secure passwords
- [ ] `bw generate --passphrase` creates passphrases
- [ ] Password options (length, character sets) work
- [ ] `bw send create` creates text Sends
- [ ] `bw send list` shows user's Sends
- [ ] `bw send get` retrieves Send details
- [ ] `bw send delete` removes Sends
- [ ] `bw receive` accesses Send content
- [ ] `bw encode` base64-encodes input
- [ ] All tests pass
- [ ] Documentation complete

**Acceptance Tests:**
1. Given default options, when running `bw generate`, then secure password returned
2. Given --length 20, when generating, then 20-character password returned
3. Given --passphrase, when generating, then word-based passphrase returned
4. Given text content, when creating Send, then Send URL returned
5. Given Send ID, when running `bw send get`, then Send details returned
6. Given Send URL, when running `bw receive`, then content retrieved
7. Given Send ID, when deleting, then Send removed
8. Given input text, when encoding, then base64 output returned
9. Given password options, when generating, then password meets requirements
10. Given expired Send, when receiving, then error returned

## Security & Safety Considerations
- Use cryptographically secure random (OsRng)
- Validate password generation parameters
- Don't log generated passwords
- Encrypt Send content properly
- Clear generated passwords from memory
- Validate Send encryption before upload
- Handle Send passwords securely
- Limit Send file sizes

## UI/UX Considerations (if applicable)
- Show generated password clearly
- Provide feedback on password strength
- Clear success messages for Send operations
- Show Send URLs prominently
- Indicate Send expiration if set
- Support --quiet for scripting
- Show progress for large Send files

## Testing Strategy
**Unit Tests:**
- Test password generation with various options
- Test passphrase generation
- Test password requirements validation
- Test Send encryption
- Test base64 encoding
- Test parameter validation

**Integration Tests:**
- Test full Send create flow
- Test Send list/get/delete
- Test receive with various Send types
- Test with test account
- Test password generation quality
- Test Send expiration handling

**Manual Test Scenarios:**
1. Generate various password types
2. Generate passphrases with different word counts
3. Create text Send
4. Create file Send (if implemented)
5. List and retrieve Sends
6. Receive Send from another account
7. Delete Send
8. Test encode command
9. Compare output with TypeScript CLI

## References & Research
- apps/cli/src/tools/commands/generate.command.ts
- apps/cli/src/tools/send/*.ts (Send commands)
- apps/cli/src/tools/commands/receive.command.ts
- apps/cli/src/tools/commands/encode.command.ts
- Bitwarden SDK generator APIs
- Password generation best practices
- Send API documentation

## Notes for PM Subagent
- Prioritize generate over Send for MVP
- Verify Send feature requirements
- Flag if file Send is required for MVP
- Confirm password generation defaults

## Notes for Architect Subagent
- Use SDK generator if available
- Design for secure random generation
- Separate Send encryption from API
- Plan for file streaming if implementing file Send
- Design generator with customization in mind
- Consider password strength validation

## Notes for Implementer Subagent
- Use rand crate with OsRng for random
- Use SDK for generation if available, otherwise implement
- Implement Send encryption using SDK
- Validate all generation parameters
- Support reading file content for Send
- Follow TypeScript output format
- Clear sensitive data from memory
- Add reasonable parameter limits

## Notes for Testing Subagent
- Test password generation quality (entropy)
- Test all character set combinations
- Test parameter boundaries
- Verify Send encryption format
- Test Send operations thoroughly
- Test receive with various Send types
- Test error handling for expired Sends
- Verify generated passwords meet requirements
- Test encode with various inputs