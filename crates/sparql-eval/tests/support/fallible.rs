// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original page-provider failures shared by fallible query and evidence controls.

use super::local_dataset;
use purrdf_core::{
    PageFault, PageGeneration, PageId, PageMaterialization, PageProvider, PagedDataset, RdfDataset,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptedFault {
    Provider,
    Cancelled,
    Deadline,
    InvalidData,
    StaleMaterialization,
    CorruptByteCharge,
}

pub struct FaultingProvider {
    pages: Box<[Arc<RdfDataset>]>,
    byte_lengths: Box<[u64]>,
    generation: PageGeneration,
    fail_page: PageId,
    fault: ScriptedFault,
    calls: AtomicUsize,
}

impl FaultingProvider {
    pub fn new(
        pages: Vec<Arc<RdfDataset>>,
        byte_lengths: Vec<u64>,
        fail_page: PageId,
        fault: ScriptedFault,
    ) -> Self {
        assert_eq!(pages.len(), byte_lengths.len());
        Self {
            pages: pages.into_boxed_slice(),
            byte_lengths: byte_lengths.into_boxed_slice(),
            generation: PageGeneration(31),
            fail_page,
            fault,
            calls: AtomicUsize::new(0),
        }
    }
}

impl PageProvider for FaultingProvider {
    fn page_count(&self) -> u64 {
        u64::try_from(self.pages.len()).unwrap()
    }

    fn generation(&self) -> PageGeneration {
        self.generation
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let Some(dataset) = self.pages.get(index) else {
            return Err(PageFault::provider(page, "page out of range"));
        };
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        let query_phase = call >= self.pages.len();
        if query_phase && page == self.fail_page {
            return match self.fault {
                ScriptedFault::Provider => Err(PageFault::provider(page, "object read failed")),
                ScriptedFault::Cancelled => Err(PageFault::cancelled(page, "cancel token set")),
                ScriptedFault::Deadline => {
                    Err(PageFault::deadline_exceeded(page, "host deadline elapsed"))
                }
                ScriptedFault::InvalidData => {
                    Err(PageFault::invalid_data(page, "page checksum mismatch"))
                }
                ScriptedFault::StaleMaterialization => Ok(PageMaterialization::new(
                    dataset.clone(),
                    PageGeneration(self.generation.0 + 1),
                    self.byte_lengths[index],
                )),
                ScriptedFault::CorruptByteCharge => Ok(PageMaterialization::new(
                    dataset.clone(),
                    self.generation,
                    self.byte_lengths[index] + 1,
                )),
            };
        }
        Ok(PageMaterialization::new(
            dataset.clone(),
            self.generation,
            self.byte_lengths[index],
        ))
    }
}

pub fn two_page_faulting_dataset(fault: ScriptedFault) -> PagedDataset {
    let pages = vec![
        local_dataset([("a", "p", "b")]),
        local_dataset([("b", "q", "c")]),
    ];
    PagedDataset::from_provider(Arc::new(FaultingProvider::new(
        pages,
        vec![10, 20],
        PageId(1),
        fault,
    )))
    .expect("fault is armed only after the successful seal")
}
