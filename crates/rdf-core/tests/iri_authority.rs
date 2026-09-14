// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The same authority grammar guards both frozen and global RDF term tables.

use purrdf_core::{GlobalDictionary, RdfDatasetBuilder};

#[test]
fn invalid_ip_literals_cannot_enter_either_rdf_dictionary() {
    for iri in [
        "http://[not-ip]/",
        "http://[1::2::3]/",
        "http://[vG.a]/",
        "http://[::ffff:256.0.0.1]/",
    ] {
        let mut global = GlobalDictionary::new();
        assert!(
            global.intern_iri(iri).is_err(),
            "global dictionary admitted {iri}"
        );
        let mut builder = RdfDatasetBuilder::new();
        builder.intern_iri(iri);
        assert!(
            builder.freeze().is_err(),
            "frozen dictionary admitted {iri}"
        );
    }
}

#[test]
fn generic_ports_and_valid_ip_literals_remain_valid_rdf_identifiers() {
    for iri in [
        "http://[2001:DB8::1]:99999/",
        "http://[Vf.address]:0000000000000000000000000000000001/",
        "http://h:999999999999999999999999999999/",
    ] {
        let mut global = GlobalDictionary::new();
        global.intern_iri(iri).expect("generic authority syntax");
        let mut builder = RdfDatasetBuilder::new();
        builder.intern_iri(iri);
        builder.freeze().expect("generic authority syntax");
    }
}
