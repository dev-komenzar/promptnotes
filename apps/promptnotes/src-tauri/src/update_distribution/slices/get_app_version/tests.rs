//! Tests for slice `get-app-version`.
//!
//! Spec: `.ori/slices/get-app-version/spec.md#test-points`.
//!
//! - domain 純粋関数 (`to_app_version`) は sibling import で検証 (TP-N / TP-F / TP-P)
//! - boundary test は public_entry (`super::get_app_version`) 経由のみ (TP-B / TP-T)

use proptest::prelude::*;

use crate::update_distribution::shared::build_info::build_version;
use crate::update_distribution::shared::types::Version;

use super::domain::to_app_version;
use super::{get_app_version, GetAppVersionQuery, GetAppVersionUseCase};

// ===== spec.md#tp-normalize =====

#[test]
fn tp_n1_semver_is_normalized_to_version_display() {
    assert_eq!(to_app_version("0.2.2").as_str(), "0.2.2");
}

#[test]
fn tp_n2_multi_digit_semver() {
    assert_eq!(to_app_version("1.10.0").as_str(), "1.10.0");
}

#[test]
fn tp_f1_pre_release_falls_back_to_raw() {
    assert_eq!(to_app_version("0.3.0-rc.1").as_str(), "0.3.0-rc.1");
}

#[test]
fn tp_f2_build_metadata_falls_back_to_raw() {
    assert_eq!(to_app_version("0.3.0+sha.abc").as_str(), "0.3.0+sha.abc");
}

#[test]
fn tp_f3_empty_string_falls_back_to_raw_without_panic() {
    assert_eq!(to_app_version("").as_str(), "");
}

#[test]
fn use_case_delegates_to_domain_normalization() {
    let v = GetAppVersionUseCase::execute(GetAppVersionQuery, "0.2.2");
    assert_eq!(v.into_string(), "0.2.2");
}

proptest! {
    /// TP-P1 (C-GAV2): `a.b.c` は `Version` の Display と一致する。
    #[test]
    fn tp_p1_semver_matches_version_display(a in any::<u32>(), b in any::<u32>(), c in any::<u32>()) {
        let raw = format!("{a}.{b}.{c}");
        let expected = Version::from_str(&raw).expect("invariant: a.b.c is parseable").to_string();
        let actual = to_app_version(&raw);
        prop_assert_eq!(actual.as_str(), expected.as_str());
    }

    /// TP-P2 (C-GAV3): parse 不能な文字列は raw のまま返る（panic しない）。
    #[test]
    fn tp_p2_unparseable_falls_back_to_raw(s in ".*") {
        prop_assume!(Version::from_str(&s).is_err());
        let actual = to_app_version(&s);
        prop_assert_eq!(actual.as_str(), s.as_str());
    }

    /// TP-P3 (C-GAV6): 同じ入力に対して冪等。
    #[test]
    fn tp_p3_idempotent(s in ".*") {
        prop_assert_eq!(to_app_version(&s), to_app_version(&s));
    }
}

// ===== spec.md#tp-boundary =====

/// TP-B1: public_entry 経由の command がビルド時定数と一致する。
#[test]
fn tp_b1_command_returns_cargo_pkg_version() {
    assert_eq!(get_app_version(), env!("CARGO_PKG_VERSION"));
}

/// TP-B2 (C-GAV5): `check_for_updates` と共有する情報源と一致する。
#[test]
fn tp_b2_command_matches_shared_build_version() {
    assert_eq!(build_version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(get_app_version(), build_version());
}

// ===== spec.md#tp-type-level =====

/// TP-T1 (C-GAV1): `fn() -> String`（`Result` を含まない）を compile-time pin。
#[test]
fn tp_t1_signature_has_no_result() {
    let f: fn() -> String = get_app_version;
    let _ = f;
}
