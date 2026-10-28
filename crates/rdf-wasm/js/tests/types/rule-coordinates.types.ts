// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: TypeScript must check the package's positional rule interfaces and
// reject coordinate-bearing variables passed to unsupported JavaScript options.
import { shaclEntail, shaclApplyRules, shaclEntailAsync, shaclApplyRulesAsync,
  shaclValidateToSarifAsync, shaclEvalNodeExprAsync, shaclProductValidateToSarifAsync,
  type AsyncShaclRuleOptions } from "@blackcatinformatics/purrdf";
const controls: AsyncShaclRuleOptions = { now: "2000-01-01T00:00:00Z", entropySeed: 0n };
shaclEntail("", "", undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, controls.now, controls.entropySeed);
shaclApplyRules("", "", undefined, undefined, undefined, true, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, controls.now, controls.entropySeed);
shaclEntailAsync("", "", undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, controls);
shaclApplyRulesAsync("", "", undefined, undefined, undefined, true, undefined, undefined, undefined, undefined, undefined, undefined, undefined, controls);
// @ts-expect-error Validation must reject coordinate-bearing variables.
shaclValidateToSarifAsync("", "", undefined, undefined, undefined, undefined, undefined, undefined, controls);
// @ts-expect-error Product validation does not execute rules.
shaclProductValidateToSarifAsync(new Uint8Array(), "", controls);
// @ts-expect-error A rule seed cannot be a string.
const invalid: AsyncShaclRuleOptions = { entropySeed: "7" };
void invalid;
// @ts-expect-error Node-expression options reject coordinate-bearing variables.
shaclEvalNodeExprAsync("", "", undefined, "<http://example.org/s>", undefined, undefined, undefined, undefined, undefined, undefined, undefined, controls);
