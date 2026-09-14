// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC 3986 authority productions, preserved through public IRI/base APIs.

use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, IriError, parse, parse_uri, terminals};

#[test]
fn every_ipv6_shape_is_validated_without_changing_lexical_identity() {
    let hosts = [
        "::",
        "::1",
        "1::",
        "1:2:3:4:5:6:7:8",
        "0001:0002:0003:0004:0005:0006:0007:0008",
        "2001:DB8::7",
        "1::2:3:4:5:6:7",
        "1:2::3:4:5:6:7",
        "1:2:3::4:5:6:7",
        "1:2:3:4::5:6:7",
        "1:2:3:4:5::6:7",
        "1:2:3:4:5:6::7",
        "1:2:3:4:5:6:7::",
        "::ffff:192.0.2.128",
        "::192.0.2.1",
        "1:2:3:4:5:6:192.0.2.1",
        "1:2:3:4:5::192.0.2.1",
    ];
    for host in hosts {
        let text = format!("http://[{host}]:000099999/dir/file");
        assert_eq!(parse(&text).expect(host).as_str(), text);
        assert_eq!(parse_uri(&text).expect(host).as_str(), text);
        let base = BaseIri::parse(&text).expect(host);
        assert_eq!(base.as_str(), text);
        let scope = BaseScope::rooted(base, BaseOrigin::Caller);
        assert_eq!(
            scope.resolve("next").expect("relative reference").as_str(),
            format!("http://[{host}]:000099999/dir/next")
        );
    }
}

#[test]
fn ipv6_rejects_invalid_structure_not_just_bad_characters() {
    for host in [
        "",
        "not-ip",
        "abcd",
        "1.2.3.4",
        ":",
        ":::1",
        "1:::2",
        "1::2::3",
        "1:2:3:4:5:6:7",
        "1:2:3:4:5:6:7:8:9",
        "1:2:3:4:5:6:7:8::",
        "00000::",
        "12345::",
        "gggg::1",
        "::ffff:256.0.0.1",
        "::ffff:192.0.2",
        "::ffff:192.0.2.1.2",
        "::ffff:192.000.2.1",
        "1:2:3:4:5:6::192.0.2.1",
        "fe80::1%25eth0",
        "::é",
        "::[1]",
    ] {
        let text = format!("http://[{host}]/");
        assert!(parse(&text).is_err(), "accepted {text:?}");
        assert!(parse_uri(&text).is_err(), "accepted URI {text:?}");
        assert!(BaseIri::parse(&text).is_err(), "accepted base {text:?}");
    }
}

#[test]
fn ipvfuture_requires_a_hex_version_dot_and_nonempty_exact_address() {
    for host in [
        "v1.a",
        "Vf.Example",
        "v0000000000000000000001.address",
        "vF.:",
        "v1..",
        "vABCD.aZ09!$&'()*+,-._~:;=",
    ] {
        let text = format!("https://[{host}]/");
        assert_eq!(parse(&text).expect(host).as_str(), text);
        assert_eq!(parse_uri(&text).expect(host).as_str(), text);
    }
    for host in [
        "v", "v1", "v.1", "v1.", "vG.a", "v１.a", "v1.é", "v1.a%20b", "v1.a@b", "v1.a b",
        "v1.a\\b", "v1.a[b",
    ] {
        let text = format!("https://[{host}]/");
        assert!(parse(&text).is_err(), "accepted {text:?}");
    }
}

#[test]
fn ipvfuture_terminal_matches_the_entire_ascii_alphabet_exactly() {
    for byte in 0_u8..=127 {
        let expected = byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:".contains(&byte);
        assert_eq!(
            terminals::is_ipvfuture_address_char(char::from(byte)),
            expected,
            "byte {byte}"
        );
    }
    for ch in ['\u{80}', '\u{a0}', 'é', '漢', '\u{10ffff}'] {
        assert!(!terminals::is_ipvfuture_address_char(ch));
    }
}

#[test]
fn authority_delimiters_and_diagnostic_offsets_stay_exact() {
    for text in [
        "http://[::1]suffix/",
        "http://[::1]:80:90/",
        "http://[::1/",
        "http://::1/",
        "http://a@b@c/",
    ] {
        assert!(parse(text).is_err(), "accepted {text:?}");
    }
    let text = "http://user@[v1.a%20b]/";
    assert_eq!(
        parse(text).expect_err("percent is forbidden in IPvFuture"),
        IriError::DisallowedChar('%', text.find('%').expect("percent"))
    );
    let text = "http://[::1]:８０/";
    assert_eq!(
        parse(text).expect_err("port is ASCII DIGIT only"),
        IriError::DisallowedChar('８', text.find('８').expect("digit"))
    );
}
