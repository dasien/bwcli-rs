# Tool Commands Enhancement - Quick Reference

## Overview
This enhancement implements password generation, Send operations, receive, and encode commands.

## Command Categories
1. **Generate** - Password and passphrase generation
2. **Send CRUD** - Create, list, get, edit, delete Sends
3. **Receive** - Access Send content by URL
4. **Encode** - Base64 encoding utility

## Current State
✅ CLI structure exists (commands/tools.rs, commands/send.rs)
✅ All dependencies complete (storage, API, auth)
❌ SDK is mocked (need real SDK for Send encryption)
❌ All implementations are stubs
❌ Receive command not yet defined in CLI

## Must Have (MVP)
- `bw generate` (password & passphrase)
- `bw send create` (text only)
- `bw send list/get/delete`
- `bw receive <url>`
- `bw encode <data>`

## Should Have (Optional)
- `bw send template`
- `bw send edit`
- `bw send remove-password`
- `bw send create --file` (file Sends)

## Critical Open Questions
1. **Does SDK provide Send encryption?** (Blocks Phase 3)
2. **Does SDK provide password generation?** (Can use `rand` as fallback)
3. **Where should receive command be?** (Recommend top-level, not under send)
4. **What's the Send file size limit?** (Need for validation)
5. **How to embed/load word list for passphrases?** (7776 words, ~60KB)

## Technical Flags for Architect
- **TF-1:** SDK generator integration decision needed
- **TF-2:** Send encryption implementation approach
- **TF-3:** File Send streaming design
- **TF-4:** Passphrase word list storage strategy
- **TF-5:** Receive command structure confirmation
- **TF-6:** Send model definitions needed

## Implementation Phases
1. Password Generation (3-5 days) - HIGH PRIORITY
2. Encode Utility (1 day) - HIGH PRIORITY
3. Send Models & Encryption (5-7 days) - HIGH PRIORITY, BLOCKED ON SDK
4. Send CRUD Commands (5-7 days) - HIGH PRIORITY
5. Receive Command (3-5 days) - HIGH PRIORITY
6. Optional Send Features (4-6 days) - MEDIUM PRIORITY
7. Integration & Polish (3-4 days) - HIGH PRIORITY

**Total Estimate:** 24-35 days for full implementation

## Security Requirements
- Use `rand::rngs::OsRng` for password generation (CSPRNG)
- Zeroize sensitive data (passwords, keys) after use
- Never log generated passwords or keys
- Use SDK encryption (don't roll custom crypto)
- Validate all inputs to prevent injection
- File size limits to prevent resource exhaustion

## Performance Targets
- Password/passphrase generation: <100ms
- Send operations: <2s
- Receive: <2s
- Encode: <100ms

## Key Dependencies
- **Bitwarden SDK** - For Send encryption (critical)
- **rand crate** - For CSPRNG (available)
- **base64 crate** - For encoding (available)
- **EFF Word List** - For passphrases (external resource)

## Risk Level: MEDIUM
- **High Risk:** SDK encryption APIs may be unavailable (mitigation: research early)
- **Medium Risk:** File Send complexity (mitigation: defer to later if needed)
- **Low Risk:** Cross-platform compatibility (mitigation: standard libraries)

## Next Steps for Architecture
1. Research Bitwarden SDK capabilities (Send encryption, generator)
2. Design Send models (`models/send/`)
3. Design Send service (`services/send/`)
4. Define receive command structure
5. Choose word list strategy
6. Resolve all open questions
7. Create detailed implementation spec
