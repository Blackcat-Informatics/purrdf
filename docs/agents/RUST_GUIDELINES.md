<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

Generic Rust style and architecture guidance for engineers and coding agents.
Repository constraints in `AGENTS.md` and the root `Cargo.toml` take precedence,
including dependency bans and workspace lints. Examples using other libraries
illustrate patterns; they do not authorize adding those dependencies.

---

# Modern Rust Engineering & Style Guide

> **Core Philosophy:** Optimize for reading, maintenance, and compile-time guarantees. Use the type system to make illegal states unrepresentable, keep external boundaries strict, and treat tests as living architecture specifications rather than coverage checkboxes.

---

## 1. Workspace & Crate Architecture

Large single-crate projects suffer from terrible incremental compile times and tangled dependencies. Structure repositories as **Cargo Workspaces** with strict topological layering.

### Recommended Repo Layout

```text
repo-root/
├── Cargo.toml              # [workspace] definition & centralized dependencies
├── crates/
│   ├── core-domain/        # Pure business logic, types, traits (NO I/O, NO async)
│   ├── infra-db/           # Database adapters implementing domain traits
│   ├── infra-http/         # Axum/Tower routers, middleware, DTOs
│   └── telemetry/          # Tracing, metrics, and non-blocking subscriber setup
└── apps/
    └── api-server/         # Thin binary (`main.rs`): wires config, telemetry, and crates

```

### Workspace Rules

* **Centralize Dependencies:** Define all third-party versions in the root `Cargo.toml` under `[workspace.dependencies]`. Individual crates must inherit them via `tokio = { workspace = true }`.
* **Thin Binaries, Thick Libraries:** `main.rs` should only parse configuration, initialize telemetry, construct dependency graphs, and spawn the top-level server or worker. Business logic belongs in library crates.
* **Strict Visibility:** Default to private. Use `pub(crate)` extensively before reaching for `pub`. Every `pub` item is a maintenance commitment.
* **No Cyclic Dependencies:** Domain crates must never know about infrastructure crates. Keep `core-domain` free of heavy dependencies (like `sqlx`, `axum`, or `reqwest`) so it compiles in milliseconds.

---

## 2. Telemetry, Logging & Observability

**BANNED:** `println!`, `eprintln!`, `dbg!`, and the legacy `log` crate in production paths. Standard output locks a global mutex and blocks the thread, which can starve async executors (`tokio`) under high load.

### Structured, Non-Blocking Tracing

Use the `tracing` ecosystem with **structured fields** and **non-blocking I/O** (`tracing-appender`).

```rust
use tracing::{info, instrument, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes non-blocking JSON telemetry.
/// MUST keep the returned `WorkerGuard` alive in `main()` until shutdown!
pub fn init_telemetry() -> WorkerGuard {
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(std::io::stdout());

    let fmt_layer = fmt::layer()
        .json() // Structured JSON for log aggregators
        .with_writer(non_blocking_writer)
        .with_target(true)
        .with_current_span(true);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt_layer)
        .init();

    guard
}

// GOOD: Skip large payloads/secrets, attach key identifiers as structured fields
#[instrument(skip(db_pool, payload), fields(user_id = %user.id, payload_len = payload.len()))]
pub async fn process_upload(db_pool: &DbPool, user: &User, payload: &[u8]) -> Result<(), AppError> {
    info!(status = "started", "Processing user upload");
    // ...
    Ok(())
}

```

### Telemetry Rules

* **Never Interpolate Strings in Logs:**
* *Bad:* `info!("User {} failed login after {} attempts", user_id, count);` (Hard to query/index).
* *Good:* `info!(%user_id, attempts = count, "User failed login");` (Emits indexed JSON fields).


* **Guard the Appender:** Always bind the `WorkerGuard` in `main` (`let _guard = init_telemetry();`). Dropping it immediately (`let _ = init_telemetry();`) silently kills background log flushing.
* **Span Discipline:** Put `#[instrument]` on meaningful boundary functions (HTTP handlers, DB queries, RPC calls), not on tiny tight-loop helpers. Always `skip` sensitive data (passwords, tokens) and large buffers.

---

## 3. SOTA Type Design & Idioms

Write code where invalid states fail to compile. Leverage zero-cost abstractions to guide both humans and AI agents.

### Parse, Don't Validate (Newtype Pattern)

Do not pass raw `String`, `u64`, or `Uuid` across module boundaries when they have domain meaning.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(uuid::Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            Err("String cannot be empty")
        } else {
            Ok(Self(value))
        }
    }
}

```

### Advanced Idioms Checklist

* **Typestate for Lifecycles:** Encode state machines into generic type parameters (e.g., `Connection<Disconnected>` vs `Connection<Authenticated>`) rather than runtime booleans (`is_connected: bool`).
* **Native Async in Traits:** Since Rust 1.75+, use native `async fn` in traits instead of the `#[async_trait]` macro, *unless* you specifically need dynamic dispatch (`Box<dyn MyTrait>`). For internal architecture, prefer static dispatch (`impl MyTrait`).
* **`let...else` for Early Returns:** Avoid deep rightward drift (`if let Some(x) = ...`).
```rust
let Some(user) = repo.find_user(id).await? else {
    return Err(DomainError::UserNotFound(id));
};

```


* **Borrowing vs. Owning:** Accept `&str` instead of `&String`, `&[T]` instead of `&Vec<T>`, and `impl AsRef<Path>` for file paths. Use `Cow<'_, str>` when a function rarely modifies its input string.
* **Enforce Safety Attributes:** Place `#[must_use]` on pure functions, builder types, and custom result types so callers cannot accidentally ignore critical return values.

---

## 4. Error Handling

Treat errors as either **recoverable by a machine** (domain/library) or **diagnostic for a human/operator** (application).

| Context | Crate | Rule |
| --- | --- | --- |
| **Libraries / Domain Crates** | `thiserror` | Define explicit, granular enums. Callers must be able to `match` on specific failure modes. |
| **Binaries / App Glue** | `anyhow` or `eyre` | Use for top-level orchestration, CLI tools, and attaching `.context("Failed to...")`. |

### Error Rules

* **BANNED in Production:** `.unwrap()` and `.expect()` are forbidden outside of tests, unless accompanied by a proof comment explaining why failure is mathematically impossible (even then, prefer `.expect("INVARIANT: ...")`).
* **Preserve the Error Chain:** Always use `#[source]` or `#[from]` in `thiserror` so the underlying root cause is never swallowed when logging.

---

## 5. High-ROI, Low-Maintenance Testing

> **The Testing Dilemma:** Every test you write has a permanent maintenance cost. Mechanical unit tests that mock every internal struct lock your implementation details in place, making refactoring painful without catching real integration bugs.

### The Testing Strategy: Test Behaviors, Not Implementations

1. **Delete Mechanical Coverage Tests:** Do not write tests for getters, setters, simple data mappers, or properties already guaranteed by the Rust compiler.
2. **Push Logic to the Pure Core:** Extract complex business rules into pure, synchronous functions in `core-domain` (takes structs in, returns structs/enums out). These require zero mocks, run in microseconds, and never flake.
3. **Property-Based Testing (`proptest`):** Instead of writing 15 copy-pasted unit tests for edge cases, write 1 property test that generates 1,000 random inputs (empty strings, Unicode, `u64::MAX`, negative numbers) and asserts invariants hold.
4. **Snapshot Testing (`insta`):** When testing complex outputs (serialized JSON, SQL query builders, ASTs, error messages), do not hand-write massive `assert_eq!` blocks. Use `insta::assert_json_snapshot!()`. When formats intentionally change, run `cargo insta review` to update all tests in seconds.
5. **Fakes over Mocks:** Avoid heavy mocking frameworks (`mockall`) that assert *how* a trait was called (`expect_get_user().times(1)`). Instead, write a lightweight in-memory Fake (e.g., `InMemoryUserRepo` backed by a `HashMap`) or test against a real ephemeral database using `sqlx::test` or `testcontainers`.

### Self-Documenting Test Structure

Every test is an executable specification. Name tests after the **behavior**, not the function name, and document the **invariant** being protected.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// INVARIANT: Applying a discount must never result in a negative subtotal
    /// or exceed the original price, regardless of coupon stacking.
    #[test]
    fn discount_calculation_clamps_between_zero_and_subtotal() {
        // Arrange
        let cart = Cart::with_subtotal(Money::from_cents(1000));
        let massive_coupon = Coupon::FixedAmount(Money::from_cents(5000));

        // Act
        let final_price = cart.apply_discount(&massive_coupon);

        // Assert
        assert_eq!(final_price, Money::ZERO);
    }

    proptest! {
        #[test]
        fn roundtrip_serialization_preserves_id(raw_id in any::<u128>()) {
            let id = OrderId::from_u128(raw_id);
            let encoded = id.to_string();
            let decoded: OrderId = encoded.parse().expect("valid format");
            prop_assert_eq!(id, decoded);
        }
    }
}

```

---

## 6. Automated Guardrails (`Cargo.toml` Lints)

Do not rely on code reviewers or AI prompts to enforce basic hygiene. Configure the compiler and Clippy in your root `Cargo.toml` so bad patterns fail the build automatically:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }

# Prevent runtime panics & blocking stdout
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"
todo = "warn"

# Allow common pedantic false-positives
module_name_repetitions = "allow"
missing_errors_doc = "allow"

```

---

## 7. Quick-Reference Rules for AI Agents

If you are an AI agent generating or refactoring code in this repository, you must adhere to these constraints before completing any task:

1. **No `println!` / `dbg!`:** Always emit structured events via `tracing::{info, debug, warn, error}`.
2. **No `.unwrap()`:** Propagate errors with `?` and map them into domain error enums. In tests, use `.expect("reason why test setup is valid")`.
3. **Check Compiler Lints:** Always run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo nextest run` (or `cargo test`) before declaring a task finished.
4. **Minimize Visibility:** Keep structs, fields, and functions private or `pub(crate)` unless explicitly required by a downstream crate.
5. **No Mechanical Tests:** When asked to write tests, test edge cases, boundary conditions, and state invariants—never write tautological tests that just mirror the implementation line-by-line.
