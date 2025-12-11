# bw-cli Code Quality Issues & Refactoring Suggestions

## 1. Duplicated Helper Functions

### Problem
`get_password_input` is defined identically in both `login.rs` and `vault_ops.rs`:

```rust
fn get_password_input(
    password_arg: Option<String>,
    global_args: &GlobalArgs,
) -> Result<Secret<String>> {
    if let Some(password) = password_arg {
        return Ok(Secret::new(password));
    }
    if global_args.nointeraction {
        anyhow::bail!(
            "Password is required. Use --nointeraction=false or provide PASSWORD argument."
        );
    }
    prompts::prompt_password(None)
}
```

### Refactoring
Move all input-gathering helpers to `prompts.rs` or create a new `input.rs` module:

```rust
// crates/bw-cli/src/commands/auth/input.rs (new file)
use crate::GlobalArgs;
use anyhow::Result;
use secrecy::Secret;
use super::prompts;

pub fn require_password(
    password_arg: Option<String>,
    global_args: &GlobalArgs,
    prompt_text: Option<&str>,
) -> Result<Secret<String>> {
    if let Some(password) = password_arg {
        return Ok(Secret::new(password));
    }
    if global_args.nointeraction {
        anyhow::bail!(
            "Password is required. Provide PASSWORD argument or disable --nointeraction."
        );
    }
    prompts::prompt_password(prompt_text)
}

pub fn require_string(
    arg: Option<String>,
    global_args: &GlobalArgs,
    field_name: &str,
    prompt_fn: impl FnOnce() -> Result<String>,
) -> Result<String> {
    if let Some(value) = arg {
        return Ok(value);
    }
    if global_args.nointeraction {
        anyhow::bail!("{} is required. Provide it as an argument or disable --nointeraction.", field_name);
    }
    prompt_fn()
}

pub fn require_secret(
    arg: Option<String>,
    global_args: &GlobalArgs,
    field_name: &str,
    prompt_fn: impl FnOnce() -> Result<Secret<String>>,
) -> Result<Secret<String>> {
    if let Some(value) = arg {
        return Ok(Secret::new(value));
    }
    if global_args.nointeraction {
        anyhow::bail!("{} is required. Provide it as an argument or disable --nointeraction.", field_name);
    }
    prompt_fn()
}
```

Usage in `login.rs`:
```rust
use super::input::{require_password, require_string, require_secret};

let email = require_string(cmd.email, global_args, "Email", prompts::prompt_email)?;
let password = require_password(cmd.password, global_args, None)?;
let client_secret = require_secret(cmd.client_secret, global_args, "Client secret", prompts::prompt_client_secret)?;
```

---

## 2. Repeated Service Initialization

### Problem
Every command handler creates its own `ServiceContainer`:

```rust
// In login.rs
let container = ServiceContainer::new(None, None, None, None)?;
let auth_service = AuthService::new(container.storage(), container.api_client());

// In vault_ops.rs
let container = ServiceContainer::new(None, None, None, None)?;
let auth_service = AuthService::new(container.storage(), container.api_client());

// In status.rs
let container = Arc::new(ServiceContainer::new(None, None, None, None)?);

// In sync.rs
let container = Arc::new(ServiceContainer::new(None, None, None, None)?);
```

### Refactoring
Create an application context initialized once and passed to handlers:

```rust
// crates/bw-cli/src/context.rs (new file)
use anyhow::Result;
use bw_core::services::ServiceContainer;
use std::sync::Arc;

pub struct AppContext {
    container: Arc<ServiceContainer>,
}

impl AppContext {
    pub fn new() -> Result<Self> {
        Ok(Self {
            container: Arc::new(ServiceContainer::new(None, None, None, None)?),
        })
    }

    pub fn container(&self) -> &Arc<ServiceContainer> {
        &self.container
    }

    pub fn storage(&self) -> Arc<dyn Storage> {
        self.container.storage()
    }

    pub fn api_client(&self) -> Arc<dyn ApiClient> {
        self.container.api_client()
    }

    pub fn sdk(&self) -> &BitwardenSdk {
        self.container.sdk()
    }
}
```

Update `main.rs`:
```rust
#[tokio::main]
async fn main() -> ExitCode {
    // ... tracing init ...
    
    let cli = Cli::parse();
    
    // Initialize context once
    let ctx = match AppContext::new() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to initialize: {:#}", e);
            return ExitCode::FAILURE;
        }
    };

    let result = execute_command(cli.command, &cli.global_args, &ctx).await;
    // ...
}

async fn execute_command(
    command: Commands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<output::Response> {
    match command {
        Login(cmd) => commands::execute_login(cmd, global_args, ctx).await,
        // ...
    }
}
```

This also enables easier testing by allowing mock implementations.

---

## 3. Inconsistent Error Handling

### Problem
Mixed patterns for handling errors:

```rust
// Pattern A: Wraps error in successful Response
Err(e) => Ok(Response::error(e.to_string())),

// Pattern B: Propagates error
Err(e) => Err(e.into()),
```

### Refactoring
Establish clear conventions:

```rust
// crates/bw-cli/src/error.rs (new file)
use crate::output::Response;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Authentication required")]
    NotAuthenticated,
    
    #[error("Vault is locked")]
    VaultLocked,
    
    #[error("Item not found: {0}")]
    NotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl CliError {
    /// Business logic errors become Response::error
    /// Infrastructure errors propagate as Err
    pub fn into_response(self) -> Result<Response, anyhow::Error> {
        match self {
            CliError::NotAuthenticated |
            CliError::VaultLocked |
            CliError::NotFound(_) |
            CliError::InvalidInput(_) => Ok(Response::error(self.to_string())),
            
            CliError::Internal(e) => Err(e),
        }
    }
}
```

Usage pattern:
```rust
pub async fn execute_get(cmd: GetCommands, global_args: &GlobalArgs, ctx: &AppContext) -> anyhow::Result<Response> {
    match cmd {
        GetCommands::Item(item_cmd) => {
            vault_service.get_item(&item_cmd.id).await
                .map(Response::success)
                .or_else(|e| CliError::from(e).into_response())
        }
        // ...
    }
}
```

---

## 4. Unnecessary Clone on Secret<String>

### Problem
```rust
let password = get_password_input(cmd.password.clone(), global_args)?;
// Later...
.login_with_password(&email, password.clone(), two_factor.clone(), None)
// And again for retry...
.login_with_password(&email, password, two_factor, Some(otp))
```

Cloning secrets creates multiple copies in memory, partially defeating the purpose of `secrecy`.

### Refactoring
Restructure to avoid clones by consuming the secret only at the final use:

```rust
pub async fn execute_password_login(
    cmd: LoginPasswordCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    let email = get_email_input(cmd.email, global_args)?;
    let password = get_password_input(cmd.password, global_args)?; // No clone on cmd.password
    
    let two_factor = cmd.code.as_ref().map(|code| TwoFactorData {
        token: code.clone(),
        provider: cmd.method.unwrap_or(TwoFactorProvider::AUTHENTICATOR),
        remember: false,
    });

    // First attempt
    let result = auth_service
        .login_with_password(&email, &password, two_factor.as_ref(), None)
        .await;

    match result {
        Ok(login_result) => Ok(format_login_success(&login_result)),
        Err(AuthError::NewDeviceVerificationRequired) => {
            if global_args.nointeraction {
                anyhow::bail!("New device verification required...");
            }
            let otp = prompts::prompt_device_verification_otp()?;
            
            // Second attempt consumes password
            let retry_result = auth_service
                .login_with_password(&email, &password, two_factor.as_ref(), Some(otp))
                .await?;
            
            Ok(format_login_success(&retry_result))
        }
        Err(e) => Err(e.into()),
    }
}
```

This requires `AuthService::login_with_password` to accept `&Secret<String>` rather than owned. If that's not possible, consider:

```rust
// Only expose/clone at the absolute point of use
use secrecy::ExposeSecret;

auth_service.login_with_password(
    &email,
    Secret::new(password.expose_secret().clone()),
    // ...
)
```

---

## 5. Magic Numbers

### Problem
```rust
provider: cmd.method.unwrap_or(0),
```

The meaning of `0` is unclear without context.

### Refactoring
Use the existing enum or define constants:

```rust
// If TwoFactorMethod enum exists, use it
use bw_core::models::auth::TwoFactorMethod;

let two_factor = cmd.code.as_ref().map(|code| TwoFactorData {
    token: code.clone(),
    provider: cmd.method
        .map(TwoFactorMethod::from_u8)
        .unwrap_or(TwoFactorMethod::Authenticator),
    remember: false,
});

// Or define constants if working with raw values
mod two_factor_provider {
    pub const AUTHENTICATOR: u8 = 0;
    pub const EMAIL: u8 = 1;
    pub const DUO: u8 = 2;
    pub const YUBIKEY: u8 = 3;
    pub const U2F: u8 = 4;
    pub const REMEMBER: u8 = 5;
    pub const ORGANIZATION_DUO: u8 = 6;
    pub const WEBAUTHN: u8 = 7;
}

provider: cmd.method.unwrap_or(two_factor_provider::AUTHENTICATOR),
```

---

## 6. Potential Panic in Index Access

### Problem
```rust
pub fn prompt_two_factor_method(available_methods: &[TwoFactorMethod]) -> Result<TwoFactorMethod> {
    // ...
    let selection = Select::new()
        .with_prompt("Two-step login method")
        .items(&methods)
        .default(0)
        .interact()?;

    Ok(available_methods[selection])  // Can panic if index out of bounds
}
```

### Refactoring
Use checked access:

```rust
pub fn prompt_two_factor_method(available_methods: &[TwoFactorMethod]) -> Result<TwoFactorMethod> {
    if available_methods.is_empty() {
        anyhow::bail!("No two-factor methods available");
    }

    let methods: Vec<&str> = available_methods.iter().map(|m| m.display_name()).collect();

    let selection = Select::new()
        .with_prompt("Two-step login method")
        .items(&methods)
        .default(0)
        .interact()?;

    available_methods
        .get(selection)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("Invalid selection index: {}", selection))
}
```

---

## 7. Unused Parameters

### Problem
```rust
pub async fn execute_lock(_cmd: LockCommand, _global_args: &GlobalArgs) -> Result<Response>
pub async fn execute_logout(_cmd: LogoutCommand, _global_args: &GlobalArgs) -> Result<Response>
```

### Refactoring
If these are intentional placeholders for future functionality, document them:

```rust
pub async fn execute_lock(
    _cmd: LockCommand,      // Reserved for future: timeout options, etc.
    _global_args: &GlobalArgs,  // Reserved for future: output format options
) -> Result<Response> {
```

Or if they'll never be used, consider simplifying the signature and updating the dispatch:

```rust
// In main.rs dispatch
Lock(_) => commands::execute_lock().await,

// In vault_ops.rs
pub async fn execute_lock() -> Result<Response> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());
    auth_service.lock().await?;
    Ok(Response::success("Your vault is locked."))
}
```

However, keeping consistent signatures across all handlers often makes the codebase more maintainable, so the `_` prefix approach is reasonable if documented.

---

## Summary Checklist

| Issue | Priority | Effort |
|-------|----------|--------|
| Duplicated helper functions | Medium | Low |
| Repeated service initialization | High | Medium |
| Inconsistent error handling | High | Medium |
| Clone on Secret | Medium | Low |
| Magic numbers | Low | Low |
| Potential panic on index | Medium | Low |
| Unused parameters | Low | Low |
