// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import init, { qualify_persistent_storage, qualify_resident_envelope } from "./qualification.js";

const output = document.querySelector("#result");
let preparation;
let linearMemory;
let phase = "preparation";
async function fetchBytes(path) {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}
try {
  const response = await fetch("./prepared.json");
  if (!response.ok) throw new Error(`prepared.json: HTTP ${response.status}`);
  const prepared = await response.json();
  preparation = prepared;
  const [source, receipt, wasm] = await Promise.all([
    fetchBytes("./source.bin"), fetchBytes("./source.receipt"), fetchBytes("./qualification_bg.wasm"),
  ]);
  const module = await init({ module_or_path: wasm });
  linearMemory = module.memory;
  const maximum = prepared.linear_memory_maximum_bytes;
  if (!Number.isSafeInteger(maximum) || maximum < 1024 * 1024 ||
      maximum > 256 * 1024 * 1024 || (maximum & (maximum - 1)) !== 0 ||
      source.byteLength !== prepared.source_bytes) {
    throw new Error("prepared workload/cap does not match the qualification envelope");
  }
  phase = "frozen_resident";
  output.textContent = "Running the frozen resident browser envelope workloads…";
  const resident = JSON.parse(qualify_resident_envelope(false));
  phase = "resident_actual_hundred_thousand";
  output.textContent = "Running the supplementary 100,100-row resident workload…";
  const actualHundredThousand = JSON.parse(qualify_resident_envelope(true));
  const supplementaryRoundtrip = actualHundredThousand.workloads?.find(workload => workload.name === "roundtrip");
  if (actualHundredThousand.groups !== 16016 || supplementaryRoundtrip?.metrics?.rdf_rows !== 100100) {
    throw new Error("supplementary resident fixture must contain exactly 100100 actual RDF rows");
  }

  phase = "persistent_storage";
  output.textContent = "Running the scan, selective join and complete streaming export…";
  const record = JSON.parse(qualify_persistent_storage(source, receipt, prepared.trusted_receipt_pin));
  const linearBytes = module.memory.buffer.byteLength;
  if (linearBytes > maximum) throw new Error("linear memory exceeded its declared maximum");
  output.textContent = JSON.stringify({
    status: "passed",
    ...record,
    resident_envelope: resident,
    resident_actual_hundred_thousand: actualHundredThousand,
    linear_memory_maximum_bytes: maximum,
    observed_linear_memory_peak_bytes: linearBytes,
    browser_user_agent: navigator.userAgent,
    hardware_concurrency: navigator.hardwareConcurrency,
    prepared,
  }, null, 2);
  output.dataset.status = "passed";
} catch (error) {
  output.textContent = JSON.stringify({
    status: "failed",
    error: String(error),
    error_stack: error?.stack,
    phase,
    observed_linear_memory_peak_bytes: linearMemory?.buffer.byteLength,
    browser_user_agent: navigator.userAgent,
    prepared: preparation,
  }, null, 2);
  output.dataset.status = "failed";
}
