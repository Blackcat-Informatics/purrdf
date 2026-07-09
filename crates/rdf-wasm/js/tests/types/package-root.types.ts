// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  ready,
  DataFactory,
  Dataset,
  type DirectionalLanguage,
  type Literal,
  type NamedNode,
  type Quad,
  type RdfTerm,
} from "@blackcatinformatics/purrdf";

await ready();

const factory = new DataFactory();
const subject: NamedNode = factory.namedNode("https://example.org/s");
const predicate: NamedNode = factory.namedNode("https://example.org/p");
const datatype: NamedNode = factory.namedNode("http://www.w3.org/2001/XMLSchema#integer");

const typed: Literal = factory.literal("7", datatype);
const language: Literal = factory.literal("hello", "en");
const direction: DirectionalLanguage = { language: "ar", direction: "rtl" };
const directional: Literal = factory.literal("مرحبا", direction);

const quoted: RdfTerm = factory.quotedTriple(subject, predicate, typed);
const quad: Quad = factory.quad(quoted, predicate, directional);

const dataset = new Dataset();
const chained: Dataset = dataset.add(quad).delete(quad).add(quad);
const rebuilt: Dataset = Dataset.from(chained);
const fromFactory: Dataset = factory.dataset(rebuilt);
const matched: Dataset = fromFactory.match(null, predicate, undefined, factory.variable("g"));

for (const item of matched) {
  const term: RdfTerm = item.object;
  term.equals(null);
  item.equals(undefined);
}

const stream: AsyncIterableIterator<Quad> = matched.toStream();
const serialized: string = matched.serialize("nquads");
const canonical: string = matched.canonicalize();
const same: boolean = matched.isomorphic(Dataset.parse(serialized, "nquads"));
const queryJson: string = matched.query("ASK { ?s ?p ?o }");

void stream;
void canonical;
void same;
void queryJson;
void language;
