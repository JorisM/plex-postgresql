//! Env-var-gated trace flags for the Maintenance-wedge debug cycle.
//!
//! Each flag reads its env var once at process start (cached via `OnceLock`)
//! so subsequent checks are cheap. Default = disabled when var unset.
//! Accepts `1` or case-insensitive `true` as enabled; everything else off.
//!
//! This module is intended for time-bounded debug use. Strip the call sites
//! together with this module once Bug #4 is identified and fixed.

use std::sync::OnceLock;

pub(crate) struct TraceFlags {
    pub txn: bool,
    pub open: bool,
    pub sql: bool,
    pub select_empty: bool,
}

static FLAGS: OnceLock<TraceFlags> = OnceLock::new();

pub(crate) fn flags() -> &'static TraceFlags {
    FLAGS.get_or_init(|| TraceFlags {
        txn: env_bool("PLEX_PG_TRACE_TXN"),
        open: env_bool("PLEX_PG_TRACE_OPEN"),
        sql: env_bool("PLEX_PG_TRACE_SQL"),
        select_empty: env_bool("PLEX_PG_TRACE_SELECT_EMPTY"),
    })
}

fn env_bool(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => v == "1" || v.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::env_bool;

    // env_bool is the only piece worth unit-testing — flags() caches via
    // OnceLock which makes per-test isolation awkward without re-architecting.

    fn with_env<F: FnOnce()>(key: &str, val: Option<&str>, body: F) {
        let original = std::env::var(key).ok();
        match val {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        body();
        match original {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn env_bool_unset_is_false() {
        with_env("PLEX_PG_TRACE_TEST_UNSET", None, || {
            assert!(!env_bool("PLEX_PG_TRACE_TEST_UNSET"));
        });
    }

    #[test]
    fn env_bool_one_is_true() {
        with_env("PLEX_PG_TRACE_TEST_ONE", Some("1"), || {
            assert!(env_bool("PLEX_PG_TRACE_TEST_ONE"));
        });
    }

    #[test]
    fn env_bool_true_case_insensitive() {
        with_env("PLEX_PG_TRACE_TEST_TRUE_MIXED", Some("TrUe"), || {
            assert!(env_bool("PLEX_PG_TRACE_TEST_TRUE_MIXED"));
        });
    }

    #[test]
    fn env_bool_zero_is_false() {
        with_env("PLEX_PG_TRACE_TEST_ZERO", Some("0"), || {
            assert!(!env_bool("PLEX_PG_TRACE_TEST_ZERO"));
        });
    }

    #[test]
    fn env_bool_arbitrary_is_false() {
        with_env("PLEX_PG_TRACE_TEST_ARBITRARY", Some("yes"), || {
            assert!(!env_bool("PLEX_PG_TRACE_TEST_ARBITRARY"));
        });
    }
}
