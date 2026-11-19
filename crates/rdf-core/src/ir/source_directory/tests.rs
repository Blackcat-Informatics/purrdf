// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use purrdf_lex::allocation::Resident;

fn digest(seed: u64) -> [u8; 32] {
    *purrdf_hash::blake3::hash(&seed.to_le_bytes()).as_bytes()
}

fn directory(digests: &[[u8; 32]], first_ordinal: u32) -> SourceDirectory {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let records = memory
        .collect(
            digests
                .iter()
                .zip(first_ordinal..)
                .map(|(&digest, ordinal)| SourceRecord {
                    digest,
                    ordinal,
                    stream: StatisticsStream::Asserted,
                }),
        )
        .unwrap();
    SourceDirectory::freeze(records, &mut memory).unwrap()
}

/// Every digest-prefix range, in the order a comparison would split them.
fn prefix_ranges(directory: &SourceDirectory) -> Vec<Range<usize>> {
    let mut pending = Vec::new();
    pending.push(0..directory.len());
    let mut ranges = Vec::new();
    while let Some(range) = pending.pop() {
        if range.is_empty() {
            continue;
        }
        ranges.push(range.clone());
        let first = directory.records[range.start].digest;
        let last = directory.records[range.end - 1].digest;
        if let Some(bit) = first_differing_bit(&first, &last) {
            let middle = partition(
                directory,
                range.clone(),
                usize::from(bit / 8),
                0x80 >> (bit % 8),
            )
            .unwrap();
            pending.push(range.start..middle);
            pending.push(middle..range.end);
        }
    }
    ranges
}

/// The sparse checkpoints answer every range a comparison asks for with the
/// node hashed from its members, reading fewer than `CHECKPOINT_ROWS` records.
#[test]
fn checkpointed_nodes_are_the_canonical_node_of_every_prefix_range() {
    for count in [0_u64, 1, 31, 32, 33, 64, 65, 257, 1000] {
        // Every seventh member repeats, so runs of equal digests are covered.
        let mut digests: Vec<_> = (0..count).map(|index| digest(index / 7 * 7)).collect();
        digests.sort_unstable();
        let directory = directory(&digests, 0);
        assert_eq!(directory.root().unwrap(), node_of(&digests));
        for range in prefix_ranges(&directory) {
            let mut work = SourceDifferenceWork::default();
            assert_eq!(
                DirectoryRead::node(&directory, range.clone(), &mut work).unwrap(),
                node_of(&digests[range.clone()]),
                "{count} members, range {range:?}"
            );
            assert!(work.members < CHECKPOINT_ROWS as u64);
        }
    }
}

fn changes(
    previous: &[&SourceDirectory],
    next: &[&SourceDirectory],
) -> (Vec<(bool, [u8; 32])>, SourceDifferenceWork) {
    let previous: Vec<_> = previous
        .iter()
        .map(|directory| (*directory).clone())
        .collect();
    let next: Vec<_> = next.iter().map(|directory| (*directory).clone()).collect();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut seen = Vec::new();
    let work = visit_directory_sets(&previous, &next, &mut memory, |_, change, _| {
        let (added, record) = change.parts();
        seen.push((added, record.digest));
        Ok::<_, StorageError>(())
    })
    .unwrap();
    seen.sort_unstable();
    (seen, work)
}

/// Range skips and root equality decide by the canonical node. A changed
/// pair whose additive sums and populations agree is visited member by
/// member; an unchanged one is skipped at its first range, also when its
/// physical addresses moved or it is spread over the same carriers.
#[test]
fn an_additive_collision_is_a_change_and_equal_members_take_the_fast_path() {
    let (removed, added) = crate::source::additive_collision();
    let shared: Vec<_> = (10..80).map(digest).collect();
    let with = |pair: &[[u8; 32]]| {
        let mut digests = shared.clone();
        digests.extend_from_slice(pair);
        digests.sort_unstable();
        digests
    };
    let before = directory(&with(&removed), 0);
    let after = directory(&with(&added), 0);
    let additive = |directory: &SourceDirectory| {
        let mut sum = Multiset256::default();
        directory
            .digests()
            .for_each(|digest| sum.insert(digest).unwrap());
        sum
    };
    assert_eq!(additive(&before), additive(&after));
    assert_ne!(before.root().unwrap(), after.root().unwrap());
    let (seen, _) = changes(&[&before], &[&after]);
    let mut expected: Vec<_> = removed
        .iter()
        .map(|digest| (false, *digest))
        .chain(added.iter().map(|digest| (true, *digest)))
        .collect();
    expected.sort_unstable();
    assert_eq!(seen, expected);

    let moved = directory(&with(&removed), 1000);
    assert_eq!(moved.root().unwrap(), before.root().unwrap());
    let (seen, work) = changes(&[&before], &[&moved]);
    assert_eq!(seen, []);
    assert_eq!(work.ranges, 1);

    let (low, high) = shared.split_at(30);
    let mut low = low.to_vec();
    low.sort_unstable();
    let mut high = high.to_vec();
    high.sort_unstable();
    let (low, high) = (directory(&low, 0), directory(&high, 0));
    let (seen, work) = changes(&[&low, &high], &[&low, &high]);
    assert_eq!(seen, []);
    assert_eq!(work.ranges, 1);
    let mut whole = shared.clone();
    whole.sort_unstable();
    let (seen, _) = changes(&[&directory(&whole, 0)], &[&low, &high]);
    assert_eq!(seen, [], "the same members over other carriers");
}

/// The public row-set identities decide by the sorted digests: a record or
/// graph set whose additive sum and population equal another's still has its
/// own resident snapshot identity and source version fingerprint. The equal
/// rows of a reordered dataset keep the resident identity (the
/// `content_identity` integration test); equal members at other physical
/// ordinals keep the version fingerprint.
#[test]
fn row_set_identities_distinguish_an_additive_collision() {
    let (removed, added) = crate::source::additive_collision();
    let with = |pair: &[[u8; 32]]| {
        let mut digests: Vec<_> = (10..40).map(digest).collect();
        digests.extend_from_slice(pair);
        digests.sort_unstable();
        digests
    };
    let (before, after) = (with(&removed), with(&added));
    let snapshot = |records: &[[u8; 32]], graphs: &[[u8; 32]]| {
        crate::source::resident_snapshot(records.iter().copied(), graphs.iter().copied(), [7; 32])
    };
    assert_ne!(snapshot(&before, &[]), snapshot(&after, &[]));
    assert_ne!(snapshot(&[], &before), snapshot(&[], &after));
    assert_eq!(
        snapshot(&before, &after),
        snapshot(&with(&removed), &with(&added))
    );

    let fingerprint = |directory: &SourceDirectory| {
        crate::source::source_version_fingerprint(
            directory.len() as u128,
            directory.root().unwrap(),
        )
    };
    let (old, new) = (directory(&before, 0), directory(&after, 0));
    assert_ne!(fingerprint(&old), fingerprint(&new));
    assert_eq!(fingerprint(&old), fingerprint(&directory(&before, 500)));
}
