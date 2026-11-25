// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: TypeScript checks unsigned host inputs and options on public package exports.
import {
  shaclPackProductWithRecursion, shaclProductValidateToSarifWithRecursion,
  shaclProductValidateToSarifAsync,
  shaclValidateChangesToSarifAsync,
  shaclValidateChangesToSarifWithRecursion,
  shaclValidateToSarifAsync,
  shaclValidateToSarifWithRecursion,
  type AsyncShaclTextValidationOptions,
} from "@blackcatinformatics/purrdf";

const limit = 18446744073709551615n;
const options: AsyncShaclTextValidationOptions = {
  recursion: "gfp",
  recursionMaxPairs: limit,
  recursionMaxIterations: 1,
};
shaclValidateToSarifWithRecursion("", "", undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, "gfp", limit, 1);
shaclValidateChangesToSarifWithRecursion("", "", undefined, undefined, undefined, undefined, undefined, undefined, undefined, "lfp", limit, 1);
shaclValidateToSarifAsync("", "", undefined, undefined, undefined, undefined, undefined, undefined, options);
shaclValidateChangesToSarifAsync("", "", undefined, undefined, undefined, undefined, undefined, undefined, options);

// @ts-expect-error Text recursion limits accept numbers and bigints.
const invalid: AsyncShaclTextValidationOptions = { recursion: "gfp", recursionMaxPairs: "1", recursionMaxIterations: 1n };
void invalid;
const product = shaclPackProductWithRecursion("", undefined, undefined, undefined, undefined, "unlimited", "gfp", limit, 1).value;
shaclProductValidateToSarifWithRecursion(product, "", false, undefined, undefined, "gfp", limit, 1);
shaclProductValidateToSarifAsync(product, "", options);
