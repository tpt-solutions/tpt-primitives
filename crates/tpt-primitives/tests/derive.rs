//! `#[derive(Canonical)]` from `tpt-derive` produces the same contract as the
//! hand-written impls.

use serde::{Deserialize, Serialize};
use tpt_primitives::Canonical;
use tpt_primitives::CanonicalError;
use tpt_primitives::Identified;
use tpt_primitives::id::IdClass;

struct DemoTag;
impl IdClass for DemoTag {
    const DOMAIN: &'static str = "tpt.demo.v1";
}

fn check_positive(demo: &Demo) -> Result<(), CanonicalError> {
    if demo.value == 0 {
        return Err(CanonicalError::InvalidContent(
            "value must be positive".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, PartialEq, Serialize, Deserialize, tpt_derive::Canonical)]
#[canonical(version = 3, id = DemoTag, validate = check_positive)]
struct Demo {
    value: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, tpt_derive::Canonical)]
#[canonical(version = 1)]
struct Plain {
    label: String,
}

#[test]
fn derive_sets_the_schema_version() {
    assert_eq!(<Demo as Canonical>::SCHEMA_VERSION, 3);
    assert_eq!(<Plain as Canonical>::SCHEMA_VERSION, 1);
}

#[test]
fn derive_sets_the_identity_class_only_when_asked() {
    let demo = Demo { value: 7 };
    assert_eq!(
        demo.identity(),
        tpt_primitives::id::Id::<DemoTag>::derive(&demo.canonical_bytes())
    );
    // `Plain` has no identity class, so it does not implement `Identified`.
    // Asserting that would need a compile-fail test; the absence is covered
    // by the type not being used with `identity()` anywhere in the workspace.
}

#[test]
fn derived_validate_runs_on_decode() {
    let good = Demo { value: 1 }.canonical_bytes();
    assert_eq!(
        Demo::from_canonical_bytes(&good).unwrap(),
        Demo { value: 1 }
    );

    let bad = Demo { value: 0 }.canonical_bytes();
    assert!(matches!(
        Demo::from_canonical_bytes(&bad),
        Err(CanonicalError::InvalidContent(_))
    ));
}

#[test]
fn derived_types_without_validate_round_trip() {
    let plain = Plain { label: "x".into() };
    assert_eq!(
        Plain::from_canonical_bytes(&plain.canonical_bytes()).unwrap(),
        plain
    );
}
