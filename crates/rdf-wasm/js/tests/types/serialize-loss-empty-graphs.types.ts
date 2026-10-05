// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: type-checks the package's TypeScript declarations with tsc.

import { Dataset, type SerializeLoss } from "@blackcatinformatics/purrdf";

const dataset = new Dataset();
const serializeLoss: SerializeLoss = dataset.serializeWithLoss("nquads");
const lossEmptyNamedGraphs: number = serializeLoss.emptyNamedGraphsDropped;

void lossEmptyNamedGraphs;
