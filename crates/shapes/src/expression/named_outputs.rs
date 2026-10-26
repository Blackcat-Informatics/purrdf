// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named-output projection shared by source and native target admission.

use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_lex::walk::WorkList;

use super::NodeExpr;
use crate::term::Term;

/// Visit every explicitly named possible IRI output. False means some output
/// still depends on execution; it does not make the visited names optional.
pub(crate) fn visit<'a, E: From<StorageError>>(
    expression: &'a NodeExpr,
    mut named: impl FnMut(&'a Term) -> Result<(), E>,
) -> Result<bool, E> {
    let mut pending = WorkList::<_, 8>::with(expression);
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut complete = true;
    while let Some(expression) = pending.pop() {
        match expression {
            NodeExpr::Constant(term @ Term::NamedNode(_)) => named(term)?,
            NodeExpr::Constant(_) | NodeExpr::Empty => {}
            NodeExpr::List(members) => {
                for member in members {
                    if matches!(member, Term::NamedNode(_)) {
                        named(member)?;
                    }
                }
            }
            NodeExpr::Union(operands) | NodeExpr::Concat(operands) => {
                for operand in operands.iter().rev() {
                    pending.try_push_admitted(operand, &mut memory)?;
                }
            }
            NodeExpr::If { then, els, .. } => {
                pending.try_push_admitted(els, &mut memory)?;
                pending.try_push_admitted(then, &mut memory)?;
            }
            NodeExpr::Owned { expr, .. } => pending.try_push_admitted(expr, &mut memory)?,
            _ => complete = false,
        }
    }
    Ok(complete)
}
