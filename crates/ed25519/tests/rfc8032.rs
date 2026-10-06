// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 8032 section 7.1: every Ed25519 test vector (TEST 1, 2, 3, 1024 and
//! SHA(abc)), checked for the public key, the exact signature bytes, strict
//! verification, and refusal of the same signature over a one-bit-different
//! message.

use purrdf_ed25519::{Signature, SignatureError, SigningKey, VerifyingKey};

struct Vector {
    name: &'static str,
    secret: &'static str,
    public: &'static str,
    message: &'static str,
    signature: &'static str,
}

fn hex<const N: usize>(text: &str) -> [u8; N] {
    purrdf_hash::hex::decode(text)
        .expect("vector hex")
        .try_into()
        .expect("vector length")
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "TEST 1",
        secret: "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        public: "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        message: "",
        signature: concat!(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bac",
            "c61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ),
    },
    Vector {
        name: "TEST 2",
        secret: "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        public: "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        message: "72",
        signature: concat!(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e",
            "458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ),
    },
    Vector {
        name: "TEST 3",
        secret: "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        public: "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        message: "af82",
        signature: concat!(
            "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290",
            "ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
        ),
    },
    Vector {
        name: "TEST 1024",
        secret: "f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5",
        public: "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e",
        message: concat!(
            "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98fa6e264bf09efe12",
            "ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d879de7c0046dc4996d9e773f4bc9efe57",
            "38829adb26c81b37c93a1b270b20329d658675fc6ea534e0810a4432826bf58c941efb65d57a338b",
            "bd2e26640f89ffbc1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe",
            "ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e06203b33890cc9b8",
            "32f79ef80560ccb9a39ce767967ed628c6ad573cb116dbefefd75499da96bd68a8a97b928a8bbc10",
            "3b6621fcde2beca1231d206be6cd9ec7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53",
            "f1d8ac70bdcb7ed185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb2",
            "d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc24554119a831a9aad6",
            "079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27088d78b7e883c8759d2c4f5c65adb7553",
            "878ad575f9fad878e80a0c9ba63bcbcc2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2",
            "cf65bd92dd597b0707e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128ba",
            "b27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51addd3b75a2b1ae078",
            "4504df543af8969be3ea7082ff7fc9888c144da2af58429ec96031dbcad3dad9af0dcbaaaf268cb8",
            "fcffead94f3c7ca495e056a9b47acdb751fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301",
            "651339e22904cc8c42f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8",
            "ca61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff7310fdc82aebfd9",
            "04b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d78ff4ef5d63653a65040cf9bfd4aca7",
            "984a74d37145986780fc0b16ac451649de6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a",
            "3ca8e1b939ae49e488acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a3",
            "2ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6aed3fb0f9cd71a4",
            "3dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb93246f6b1116398a346f1a641f3b041",
            "e989f7914f90cc2c7fff357876e506b50d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d9",
            "0d11faa933a15ef1369546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380d",
            "b2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0618983f8741c5ef",
            "68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d0",
        ),
        signature: concat!(
            "0aab4c900501b3e24d7cdf4663326a3a87df5e4843b2cbdb67cbf6e460fec350aa5371b1508f9f45",
            "28ecea23c436d94b5e8fcd4f681e30a6ac00a9704a188a03",
        ),
    },
    Vector {
        name: "SHA(abc)",
        secret: "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
        public: "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
        message: concat!(
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a",
            "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        ),
        signature: concat!(
            "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b589",
            "09351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
        ),
    },
];

fn every_section_7_1_vector_reproduces_and_verifies() {
    for v in VECTORS {
        let key = SigningKey::from_bytes(&hex(v.secret));
        let message = purrdf_hash::hex::decode(v.message).expect("message hex");
        let expected: [u8; 64] = hex(v.signature);
        assert_eq!(
            key.verifying_key().to_bytes(),
            hex::<32>(v.public),
            "{} public key",
            v.name
        );
        let signature = key.sign(&message);
        assert_eq!(signature.to_bytes(), expected, "{} signature", v.name);

        let public = VerifyingKey::from_bytes(&hex(v.public)).expect("vector key decodes");
        assert_eq!(public, key.verifying_key());
        assert_eq!(
            public.verify_strict(&message, &Signature::from_bytes(&expected)),
            Ok(()),
            "{}",
            v.name
        );
        assert_eq!(
            public.verify(&message, &Signature::from_bytes(&expected)),
            Ok(()),
            "{}",
            v.name
        );
    }
}

fn every_section_7_1_signature_is_refused_over_a_changed_message() {
    for v in VECTORS {
        let public = VerifyingKey::from_bytes(&hex(v.public)).expect("vector key decodes");
        let signature = Signature::from_bytes(&hex(v.signature));
        let mut message = purrdf_hash::hex::decode(v.message).expect("message hex");
        // One appended byte for the empty message, one flipped bit otherwise.
        match message.first_mut() {
            Some(first) => *first ^= 1,
            None => message.push(0),
        }
        assert_eq!(
            public.verify_strict(&message, &signature),
            Err(SignatureError::Mismatch),
            "{}",
            v.name
        );
    }
}

fn shared_clearing_preserves_live_array_and_ed25519_api() {
    let mut bytes = [0xa5u8; 64];
    purrdf_ed25519::wipe_secret(&mut bytes);
    assert_eq!(bytes, [0; 64]);
    let mut words = [-3i32; 256];
    purrdf_ed25519::wipe_secret(&mut words);
    assert_eq!(words, [0; 256]);
    let mut owner = purrdf_hash::SecretArray::new([u64::MAX; 25]);
    let mut clone = owner.clone();
    owner.clear();
    assert_eq!(*owner, [0; 25]);
    assert_eq!(*clone, [u64::MAX; 25]);
    clone.clear();
    assert_eq!(*clone, [0; 25]);
}

purrdf_testkit::harness_main!(
    every_section_7_1_vector_reproduces_and_verifies,
    every_section_7_1_signature_is_refused_over_a_changed_message,
    shared_clearing_preserves_live_array_and_ed25519_api,
);
