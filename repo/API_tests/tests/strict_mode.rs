//! Strict-mode behaviour for the integration test helpers.
//!
//! The default mode for `skip_if_offline` / `setup` is "skip with a log"
//! so `cargo test --workspace` on a fresh checkout stays green.
//! `API_TESTS_STRICT=1` flips this — missing prerequisites become hard
//! failures so CI can enforce that the suite actually ran.
//!
//! These tests validate the toggle by reading the env var directly; they
//! do not spawn sub-processes or rely on a live backend.

use api_tests::strict_mode;

#[test]
fn strict_mode_defaults_to_off() {
    // Only run the default-off check when the env var is explicitly
    // unset or false. Don't fail when a caller deliberately set it.
    match std::env::var("API_TESTS_STRICT") {
        Ok(v) if v == "1" || v == "true" || v == "TRUE" || v == "yes" => {
            assert!(
                strict_mode(),
                "strict_mode() should be true when env says so"
            );
        }
        _ => {
            assert!(!strict_mode(), "strict_mode() should default to false");
        }
    }
}

#[test]
fn strict_mode_recognises_truthy_env_values() {
    // Scoped env var sets must not leak across tests — Rust's test harness
    // runs tests in the same process by default, so we restore the value
    // before returning.
    let prior = std::env::var("API_TESTS_STRICT").ok();

    for truthy in ["1", "true", "TRUE", "yes"] {
        std::env::set_var("API_TESTS_STRICT", truthy);
        assert!(
            strict_mode(),
            "strict_mode() must be true for env value `{truthy}`"
        );
    }
    for falsy in ["0", "false", "", "no", "strict"] {
        std::env::set_var("API_TESTS_STRICT", falsy);
        assert!(
            !strict_mode(),
            "strict_mode() must be false for env value `{falsy}`"
        );
    }

    // Restore.
    match prior {
        Some(v) => std::env::set_var("API_TESTS_STRICT", v),
        None => std::env::remove_var("API_TESTS_STRICT"),
    }
}
