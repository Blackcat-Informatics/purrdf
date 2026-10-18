// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Missing compiler licenses refuse instead of publishing opaque source support.

use super::{Error, Role};
use crate::{owl_dl::Kb, reasoner::schema_tests::Fixture};
use purrdf_iri::vocab::{owl, rdfs};
use purrdf_lex::allocation::{Memory, Resident};

#[test]
fn composite_order_license_is_explicitly_missing_not_empty_success() {
    let mut f = Fixture::default();
    let p = f.name("p");
    let q = f.name("q");
    let r = f.name("r");
    let x = f.name("x");
    let t = f.name("t");
    f.edge(p, rdfs::SUB_PROPERTY_OF, q);
    f.edge(q, rdfs::SUB_PROPERTY_OF, r);
    let list = f.list(&[r, x]);
    f.edge(t, owl::PROPERTY_CHAIN_AXIOM, list);
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let program = kb.role_program.as_ref().unwrap();
    let p = Role::Named(kb.interner.id_of_iri("https://example.org/p").unwrap());
    let r = Role::Named(kb.interner.id_of_iri("https://example.org/r").unwrap());
    let transition = program
        .machine(r)
        .unwrap()
        .transitions
        .iter()
        .position(|transition| transition.letter == Some(p))
        .unwrap();
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    assert_eq!(
        program.path_inputs([(r, transition)], None, &mut memory),
        Err(Error::MissingOrder { sub: p, sup: r })
    );
    assert_eq!(memory.admitted_bytes(), 0);
    assert_eq!(
        program.path_inputs([(r, usize::MAX)], None, &mut memory),
        Err(Error::MissingTransition)
    );
    assert_eq!(memory.admitted_bytes(), 0);
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    assert_eq!(
        program.path_inputs([], Some(&Stop), &mut memory),
        Err(Error::Stopped)
    );
    assert_eq!(memory.admitted_bytes(), 0);
}
