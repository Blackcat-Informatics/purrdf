// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native word identity laws: strict reversible full forms, checked selectors,
//! transactional publication and the finite field's claimed error detection.

use purrdf_hash::{
    blake3,
    hex::Digest32,
    word_id::{self, Error, Reference},
};

fn sample(index: u64) -> Digest32 {
    Digest32::new(*blake3::hash(&index.to_le_bytes()).as_bytes())
}

fn frozen_alphabet_and_full_prefix_vectors() {
    use purrdf_testkit::vectors::{VectorFile, decode_str};
    let alphabet = include_str!("../../hash/data/word-id/alphabet.txt");
    let actual = (0..4096)
        .map(|index| {
            let word = word_id::dictionary_word(index).unwrap();
            assert_eq!(word_id::dictionary_index(word.as_bytes()), Some(index));
            word
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(actual, alphabet);
    assert_eq!(
        purrdf_hash::hex::encode(blake3::hash(alphabet.as_bytes()).as_bytes()),
        "39a51de1fc00b10225ceeae54f6a7c1cd3ad7f191af0f92f5eb044132c534c05"
    );
    let vectors = VectorFile::parse(include_str!("vectors/word_id.txt")).unwrap();
    for record in vectors.records() {
        let digest = Digest32::from_hex(record.fields[0]).unwrap();
        let payload = record.fields[1].parse().unwrap();
        let text = decode_str(record.fields[2]).unwrap();
        let reference = Reference::from_digest(digest, payload).unwrap();
        assert_eq!(reference.to_text().unwrap(), text);
        assert_eq!(word_id::decode(text.as_bytes()), Ok(reference));
    }
}

fn complete_roundtrips_prefix_bits_and_order() {
    let mut ordered = Vec::new();
    for index in 0..512 {
        let digest = sample(index);
        let full = Reference::Full(digest);
        let text = full.to_text().unwrap();
        assert_eq!(text.len(), full.encoded_len());
        assert_eq!(word_id::decode(text.as_bytes()), Ok(full));
        let mut output = [99; 32];
        word_id::decode_full(text.as_bytes(), &mut output).unwrap();
        assert_eq!(output, digest.into_bytes());
        ordered.push((digest, text));
        for length in 1..=21 {
            let reference = Reference::from_digest(digest, length).unwrap();
            let Reference::Prefix(prefix) = reference else {
                panic!("prefix requested")
            };
            let text = reference.to_text().unwrap();
            assert_eq!(word_id::decode(text.as_bytes()), Ok(reference));
            assert_eq!(prefix.bit_len(), length * 12);
            assert!(prefix.matches(digest));
            let mut altered = digest.into_bytes();
            altered[(length * 12 - 1) / 8] ^= 1 << (7 - (length * 12 - 1) % 8);
            assert!(!prefix.matches(Digest32::new(altered)));
        }
    }
    ordered.sort_unstable();
    assert!(ordered.windows(2).all(|pair| pair[0].1 < pair[1].1));
    let dictionary: Vec<_> = (0..4096)
        .map(|index| word_id::dictionary_word(index).unwrap())
        .collect();
    assert!(dictionary.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        dictionary
            .iter()
            .all(|word| word.bytes().all(|byte| byte.is_ascii_lowercase()))
    );
}

fn canonical_refusals_and_transactional_batches() {
    assert_eq!(word_id::decode(b"b3p2: "), Err(Error::Header { offset: 3 }));
    assert_eq!(
        word_id::decode(b"b3p1: A"),
        Err(Error::Syntax { offset: 6 })
    );
    assert_eq!(
        word_id::decode(b"b3p1: zzzzzzzzzz"),
        Err(Error::UnknownWord { offset: 6, word: 0 })
    );
    let full = Reference::Full(sample(7));
    let text = full.to_text().unwrap();
    let mut buffer = [0xa5; word_id::MAX_ENCODED_LEN];
    let original = buffer;
    assert!(
        matches!(full.encode(&mut buffer[..text.len()-1]), Err(Error::Capacity { needed, .. }) if needed == text.len())
    );
    assert_eq!(buffer, original);
    let len = full.encode(&mut buffer).unwrap();
    assert_eq!(&buffer[..len], text.as_bytes());
    assert_eq!(&buffer[len..], &original[len..]);
    for invalid in [
        text.replacen("b3w1", "b3w2", 1),
        text.replacen(' ', "  ", 1),
        format!(" {text}"),
        format!("{text} "),
        text.to_uppercase(),
        text.replacen(' ', "\t", 1),
        text.replacen(' ', "\u{a0}", 1),
    ] {
        let mut output = [0xa5; 32];
        assert!(word_id::decode_full(invalid.as_bytes(), &mut output).is_err());
        assert_eq!(output, [0xa5; 32]);
    }
    let prefix = Reference::from_digest(sample(7), 1)
        .unwrap()
        .to_text()
        .unwrap();
    assert_eq!(
        word_id::decode_full(prefix.as_bytes(), &mut [0; 32]),
        Err(Error::ExpectedFull)
    );
    let inputs = [text.as_bytes(), b"invalid", prefix.as_bytes()];
    let results: Vec<_> = word_id::decode_batch(inputs).collect();
    assert_eq!(results[0], Ok(full));
    assert_eq!(results[1].unwrap_err().record, 1);
    assert!(matches!(results[2], Ok(Reference::Prefix(_))));
    let mut seen = 0;
    word_id::encode_batch([full, full], &mut buffer, |record, result| {
        assert_eq!(record, seen);
        seen += 1;
        assert_eq!(result.unwrap(), text.as_bytes());
        Ok::<_, core::convert::Infallible>(())
    })
    .unwrap();
    assert_eq!(seen, 2);
}

// Independent carry-less polynomial multiplication and long reduction, rather
// than the production shift recurrence. Degree 12 modulus is 0x1053.
fn multiply(left: u16, right: u16) -> u16 {
    let mut product = 0_u32;
    for bit in 0..12 {
        if right & (1 << bit) != 0 {
            product ^= u32::from(left) << bit;
        }
    }
    for bit in (12..=22).rev() {
        if product & (1 << bit) != 0 {
            product ^= 0x1053 << (bit - 12);
        }
    }
    product as u16
}

fn field_weights_and_every_displayed_error_position() {
    let mut power = 1;
    let mut seen = [false; 4096];
    for _ in 0..4095 {
        assert!(!seen[usize::from(power)]);
        seen[usize::from(power)] = true;
        power = multiply(power, 2);
    }
    assert_eq!(power, 1);
    assert!(seen[1..].iter().all(|value| *value));
    let text = Reference::Full(sample(99)).to_text().unwrap();
    let words: Vec<_> = text[6..].split(' ').collect();
    let indices: Vec<_> = words
        .iter()
        .map(|word| {
            (0..4096)
                .find(|index| word_id::dictionary_word(*index) == Some(*word))
                .unwrap()
        })
        .collect();
    let mut check = multiply(0xb31 ^ 0x16, 2);
    for digit in &indices[..22] {
        check = multiply(check ^ digit, 2);
    }
    assert_eq!(check, indices[22]);
    for position in 0..23 {
        let mut changed = words.clone();
        changed[position] = word_id::dictionary_word(indices[position] ^ 1).unwrap();
        assert!(
            word_id::decode(format!("b3w1: {}", changed.join(" ")).as_bytes()).is_err(),
            "substitution {position}"
        );
        let other = (position + 1..position + 23)
            .map(|index| index % 23)
            .find(|index| words[*index] != words[position])
            .unwrap();
        changed.clone_from(&words);
        changed.swap(position, other);
        assert!(
            word_id::decode(format!("b3w1: {}", changed.join(" ")).as_bytes()).is_err(),
            "swap {position}/{other}"
        );
    }
    // Check-involving swaps at every payload position, beside their valid input.
    for position in 0..22 {
        if words[position] != words[22] {
            let mut changed = words.clone();
            changed.swap(position, 22);
            assert!(word_id::decode(format!("b3w1: {}", changed.join(" ")).as_bytes()).is_err());
        }
    }
    // Prefix forms use the same public parser but a different seed/count.
    for payload in 1..=21 {
        let text = Reference::from_digest(sample(payload as u64), payload)
            .unwrap()
            .to_text()
            .unwrap();
        let mut words: Vec<_> = text[6..].split(' ').collect();
        let current = (0..4096)
            .find(|index| word_id::dictionary_word(*index) == Some(words[payload]))
            .unwrap();
        words[payload] = word_id::dictionary_word(current ^ 1).unwrap();
        assert!(matches!(
            word_id::decode(format!("b3p1: {}", words.join(" ")).as_bytes()),
            Err(Error::Check { .. })
        ));
    }
}

purrdf_testkit::harness_main!(
    frozen_alphabet_and_full_prefix_vectors,
    complete_roundtrips_prefix_bits_and_order,
    canonical_refusals_and_transactional_batches,
    field_weights_and_every_displayed_error_position
);
