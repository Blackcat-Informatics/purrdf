// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Compile-only checks for the dated XPath regex law option (`npm run typecheck`).
// Nothing here runs.
// Why not Rust: checks the published TypeScript declarations that npm consumers compile against, which only tsc can type-check

import {
  Dataset,
  QueryEngine,
  shaclProductValidateToSarif,
  shaclApplyRules,
  shaclApplyRulesAsync,
  shaclEntail,
  shaclEntailAsync,
  shaclEvalNodeExpr,
  shaclEvalNodeExprAsync,
  shaclProductValidateToSarifAsync,
  shaclValidateChangesToSarifAsync,
  shaclValidateToSarif,
  type AsyncShaclValidationOptions,
  type GovernedQueryOptions,
  type QueryOptions,
  type XPathRegexLaw,
} from "@blackcatinformatics/purrdf";

const engine = new QueryEngine();
const dataset = new Dataset();
const laws: XPathRegexLaw[] = ["xpath-2.0-2010-12-14", "xpath-3.1-2017-03-21"];
const selected: QueryOptions = { xpathRegex: "xpath-3.1-2017-03-21" };
const governed: GovernedQueryOptions = { xpathRegex: "xpath-2.0-2010-12-14", fuel: 10 };
const asked: boolean = engine.ask(dataset, "ASK {}", selected);
const outcome = engine.queryGoverned(dataset, "ASK {}", governed);
const raw: string = dataset.query("ASK {}", null, "xpath-3.1-2017-03-21");
const unselected: string = dataset.query("ASK {}");
const sarif: string = shaclValidateToSarif(
  "",
  "",
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  "xpath-3.1-2017-03-21",
);
const product: string = shaclProductValidateToSarif(new Uint8Array(), "", "xpath-2.0-2010-12-14");
const validation: AsyncShaclValidationOptions = {
  xpathRegex: "xpath-3.1-2017-03-21",
  yieldEveryPolls: 0,
};
const asyncProduct: Promise<string> = shaclProductValidateToSarifAsync(
  new Uint8Array(),
  "",
  validation,
);
const asyncChange = shaclValidateChangesToSarifAsync("", "", null, null, null, null, null, null, {
  xpathRegex: null,
});

const entailed = shaclEntail(
  "",
  "",
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  "xpath-3.1-2017-03-21",
);
const inferred = shaclApplyRules(
  "",
  "",
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  "xpath-2.0-2010-12-14",
);
const evaluated = shaclEvalNodeExpr(
  "",
  "",
  "_:e",
  "\"aa\"",
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  undefined,
  "xpath-3.1-2017-03-21",
);
const asyncEntailed = shaclEntailAsync("", "", null, null, null, null, null, null, null, null, validation);
const asyncInferred = shaclApplyRulesAsync(
  "",
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  validation,
);
const asyncEvaluated = shaclEvalNodeExprAsync(
  "",
  "",
  "_:e",
  "\"aa\"",
  null,
  null,
  null,
  null,
  null,
  null,
  null,
  validation,
);
// @ts-expect-error the tools' law is a dated name too
shaclEntail("", "", undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, "xpath-3.1");

// @ts-expect-error an undated name selects no law
const undated: QueryOptions = { xpathRegex: "xpath-3.1" };
// @ts-expect-error names are matched exactly, with no case folding
const shouted: QueryOptions = { xpathRegex: "XPATH-3.1-2017-03-21" };

void [laws, asked, outcome, raw, unselected, sarif, product, asyncProduct, asyncChange];
void [undated, shouted];
void [entailed, inferred, evaluated, asyncEntailed, asyncInferred, asyncEvaluated];
