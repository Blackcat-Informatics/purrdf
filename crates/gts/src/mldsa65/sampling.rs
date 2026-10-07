// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 204 Algorithms 29–34; finite rejection budgets fail explicitly.

use super::math::{GAMMA1, K, L, N, Poly, Q};
use super::{Error, SecretPolys, codec};
use purrdf_hash::sha3::{Shake128, Shake256, ShakeReader};
use purrdf_hash::{SecretArray, wipe_secret};

// More than a million candidates per polynomial/challenge. This bounds even
// malicious public inputs, with negligible failure probability for SHAKE.
const CANDIDATE_LIMIT: usize = 1 << 20;

pub(super) fn hash(parts: &[&[u8]], out: &mut [u8]) {
    let mut sponge = Shake256::new();
    for part in parts {
        sponge.update(part);
    }
    sponge.finalize().squeeze(out);
}

pub(super) fn matrix(rho: &[u8]) -> Result<Vec<Poly>, Error> {
    let mut matrix = vec![[0; N]; K * L];
    for row in 0..K {
        for column in 0..L {
            let mut sponge = Shake128::new();
            sponge.update(rho);
            sponge.update(&[column as u8, row as u8]);
            uniform(
                &mut sponge.finalize(),
                &mut matrix[row * L + column],
                CANDIDATE_LIMIT,
            )?;
        }
    }
    Ok(matrix)
}

fn uniform(reader: &mut ShakeReader<128>, out: &mut Poly, budget: usize) -> Result<(), Error> {
    let mut bytes = SecretArray::new([0; 3]);
    let result = (|| {
        let mut index = 0;
        for _ in 0..budget {
            reader.squeeze(&mut bytes[..]);
            let candidate = i32::from(bytes[0])
                + (i32::from(bytes[1]) << 8)
                + (i32::from(bytes[2] & 127) << 16);
            bytes.clear();
            if candidate < Q {
                out[index] = candidate;
                index += 1;
                if index == N {
                    return Ok(());
                }
            }
        }
        Err(Error::SamplingExhausted)
    })();
    bytes.clear();
    #[cfg(test)]
    assert_eq!(*bytes, [0; 3]);
    if result.is_err() {
        wipe_secret(out);
    }
    result
}

pub(super) fn secrets(seed: &[u8]) -> Result<SecretPolys, Error> {
    let mut secret = SecretPolys::zeros(L + K);
    for (index, poly) in secret.0.iter_mut().enumerate() {
        let mut sponge = Shake256::new();
        sponge.update(seed);
        sponge.update(&(index as u16).to_le_bytes());
        bounded(&mut sponge.finalize(), poly, CANDIDATE_LIMIT)?;
    }
    Ok(secret)
}

fn bounded(reader: &mut ShakeReader<256>, out: &mut Poly, budget: usize) -> Result<(), Error> {
    let mut byte = SecretArray::new([0]);
    let result = (|| {
        let mut index = 0;
        for _ in 0..budget {
            reader.squeeze(&mut byte[..]);
            let mut candidates = SecretArray::new([byte[0] & 15, byte[0] >> 4]);
            byte.clear();
            for candidate in candidates.iter().copied() {
                if candidate < 9 {
                    out[index] = 4 - i32::from(candidate);
                    index += 1;
                    if index == N {
                        break;
                    }
                }
            }
            candidates.clear();
            #[cfg(test)]
            assert_eq!(*candidates, [0; 2]);
            if index == N {
                return Ok(());
            }
        }
        Err(Error::SamplingExhausted)
    })();
    byte.clear();
    #[cfg(test)]
    assert_eq!(*byte, [0]);
    if result.is_err() {
        wipe_secret(out);
    }
    result
}

pub(super) fn mask(seed: &[u8], nonce: u16) -> SecretPolys {
    let mut polys = SecretPolys::zeros(L);
    let mut bytes = super::SecretBytes::zeros(codec::RESPONSE_BYTES);
    for (index, poly) in polys.0.iter_mut().enumerate() {
        hash(&[seed, &(nonce + index as u16).to_le_bytes()], &mut bytes.0);
        codec::unpack(&bytes.0, 20, Some(GAMMA1), poly);
    }
    polys
}

pub(super) fn challenge(seed: &[u8]) -> Result<SecretPolys, Error> {
    challenge_with_budget(seed, CANDIDATE_LIMIT)
}

fn challenge_with_budget(seed: &[u8], budget: usize) -> Result<SecretPolys, Error> {
    let mut sponge = Shake256::new();
    sponge.update(seed);
    let mut reader = sponge.finalize();
    let mut sign_bytes = SecretArray::new([0; 8]);
    reader.squeeze(&mut sign_bytes[..]);
    let mut signs = SecretArray::new([u64::from_le_bytes(*sign_bytes)]);
    sign_bytes.clear();
    #[cfg(test)]
    assert_eq!(*sign_bytes, [0; 8]);
    drop(sign_bytes);
    let mut byte = SecretArray::new([0]);
    let result = (|| {
        let mut challenge = SecretPolys::zeros(1);
        let poly = &mut challenge.0[0];
        let mut attempts = 0;
        for index in N - 49..N {
            let position = loop {
                if attempts == budget {
                    return Err(Error::SamplingExhausted);
                }
                attempts += 1;
                reader.squeeze(&mut byte[..]);
                let position = usize::from(byte[0]);
                byte.clear();
                if position <= index {
                    break position;
                }
            };
            // Challenge positions depend on the commitment hash, which is part of
            // the public signature, not on a private polynomial coefficient.
            poly[index] = poly[position];
            poly[position] = 1 - 2 * ((signs[0] >> (index + 49 - N)) & 1) as i32;
        }
        Ok(challenge)
    })();
    drop(reader);
    signs.clear();
    byte.clear();
    #[cfg(test)]
    {
        assert_eq!(*signs, [0]);
        assert_eq!(*byte, [0]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_budget_never_returns_a_partial_polynomial() {
        let mut public = [0; N];
        let mut secret = SecretPolys::zeros(1);
        let mut uniform_reader = Shake128::new().finalize();
        let mut bounded_reader = Shake256::new().finalize();
        assert_eq!(
            uniform(&mut uniform_reader, &mut public, 1),
            Err(Error::SamplingExhausted)
        );
        assert_eq!(
            bounded(&mut bounded_reader, &mut secret.0[0], 1),
            Err(Error::SamplingExhausted)
        );
        assert!(matches!(
            challenge_with_budget(&[0; 48], 1),
            Err(Error::SamplingExhausted)
        ));
        assert_eq!(public, [0; N]);
        assert_eq!(secret.0[0], [0; N]);
    }

    #[test]
    fn sampler_success_and_partial_failure_clean_owned_scratch() {
        for budget in [0, 1, 17, CANDIDATE_LIMIT] {
            let mut public = [0xa5; N];
            let mut private = [0xa5; N];
            let uniform_result = uniform(&mut Shake128::new().finalize(), &mut public, budget);
            let bounded_result = bounded(&mut Shake256::new().finalize(), &mut private, budget);
            if budget == CANDIDATE_LIMIT {
                assert_eq!(uniform_result, Ok(()));
                assert_eq!(bounded_result, Ok(()));
                assert!(public.iter().all(|&x| (0..Q).contains(&x)));
                assert!(private.iter().all(|&x| (-4..=4).contains(&x)));
                assert!(challenge_with_budget(&[0xa5; 48], budget).is_ok());
            } else {
                assert_eq!(uniform_result, Err(Error::SamplingExhausted));
                assert_eq!(bounded_result, Err(Error::SamplingExhausted));
                assert_eq!(public, [0; N]);
                assert_eq!(private, [0; N]);
                assert!(matches!(
                    challenge_with_budget(&[0xa5; 48], budget),
                    Err(Error::SamplingExhausted)
                ));
            }
        }
    }
}
