// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native proofs that SHACL bundle admission preserves the required XPath law.

use purrdf_core::xsd_regex::xpath::{self, Error, Limits, Profile, Resource};
use purrdf_shapes::ShaclProfile;

const BUNDLES: [(ShaclProfile, Profile); 2] = [
    (ShaclProfile::REC_20170720, Profile::Xpath20),
    (ShaclProfile::WD_20260918, Profile::Xpath31),
];

#[test]
fn dated_bundles_resolve_to_their_native_laws_and_finite_limits() {
    assert_eq!(ShaclProfile::LEGACY.xpath_profile(), None);
    assert_eq!(ShaclProfile::LEGACY.resolve_xpath(None).unwrap(), None);
    for (shacl, xpath) in BUNDLES {
        assert_eq!(shacl.xpath_profile(), Some(xpath));
        assert_eq!(
            shacl.resolve_xpath(None).unwrap(),
            Some((xpath, Limits::default()))
        );
        let (law, limits) = shacl.resolve_xpath(None).unwrap().unwrap();
        let program = xpath::compile(law, "(a)\\1", "", limits).unwrap();
        assert_eq!(program.profile(), Some(xpath));
        assert_eq!(program.source(), "(a)\\1");
    }
}

#[test]
fn incompatible_override_is_typed_and_the_same_law_is_an_admitted_neighbor() {
    for (shacl, required) in BUNDLES {
        let requested = if required == Profile::Xpath20 {
            Profile::Xpath31
        } else {
            Profile::Xpath20
        };
        let limits = Limits::default().with(Resource::MatchSteps, 7);
        let refusal = shacl
            .resolve_xpath(Some((requested, limits)))
            .expect_err("a dated bundle cannot substitute another regex law");
        assert_eq!(refusal.profile(), shacl);
        assert_eq!(refusal.required(), required);
        assert_eq!(refusal.requested(), requested);
        assert_eq!(
            shacl.resolve_xpath(Some((required, limits))).unwrap(),
            Some((required, limits))
        );
        assert_eq!(
            ShaclProfile::LEGACY
                .resolve_xpath(Some((requested, limits)))
                .unwrap(),
            Some((requested, limits)),
            "the compatibility law makes no dated XPath claim"
        );
    }
}

#[test]
fn current_source_limits_are_not_replaced_by_a_bundles_default() {
    for (shacl, law) in BUNDLES {
        let limits = Limits::default().with(Resource::PatternBytes, 0);
        let (selected, admitted_limits) =
            shacl.resolve_xpath(Some((law, limits))).unwrap().unwrap();
        match xpath::compile(selected, "a", "", admitted_limits).unwrap_err() {
            Error::Resource(refusal) => {
                assert_eq!(refusal.resource, Resource::PatternBytes);
                assert_eq!(refusal.limit, 0);
                assert_eq!(refusal.required, 1);
            }
            error => panic!("expected typed source refusal, got {error:?}"),
        }
        let limits = limits.with(Resource::PatternBytes, 1);
        let (selected, admitted_limits) =
            shacl.resolve_xpath(Some((law, limits))).unwrap().unwrap();
        xpath::compile(selected, "a", "", admitted_limits)
            .expect("one admitted byte is a valid neighboring request");
    }
}

#[test]
fn grammar_differences_are_executed_by_the_selected_native_home() {
    let (rec, rec_limits) = ShaclProfile::REC_20170720
        .resolve_xpath(None)
        .unwrap()
        .unwrap();
    let (draft, draft_limits) = ShaclProfile::WD_20260918
        .resolve_xpath(None)
        .unwrap()
        .unwrap();
    assert!(matches!(
        xpath::compile(rec, "(?:a)", "", rec_limits),
        Err(Error::Syntax { .. })
    ));
    xpath::compile(draft, "(?:a)", "", draft_limits).unwrap();
    assert!(matches!(
        xpath::compile(rec, "a", "q", rec_limits),
        Err(Error::Flags { .. })
    ));
    xpath::compile(draft, "a", "q", draft_limits).unwrap();
    xpath::compile(rec, "(a)", "", rec_limits).unwrap();
}
