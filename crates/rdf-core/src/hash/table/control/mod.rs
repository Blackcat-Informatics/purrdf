// SPDX-FileCopyrightText: 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT OR Apache-2.0
// Modified for PurRDF. Local changes and their copyright/grant are recorded
// in crates/rdf-core/src/hash/table/NOTICE and PROVENANCE.md.

mod bitmask;
mod group;
mod tag;

use self::bitmask::BitMask;
pub(crate) use self::{
    bitmask::BitMaskIter,
    group::Group,
    tag::{Tag, TagSliceExt},
};
