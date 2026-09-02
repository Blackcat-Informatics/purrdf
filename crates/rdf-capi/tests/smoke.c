/* SPDX-License-Identifier: MIT OR Apache-2.0 */
/* A C smoke test for libpurrdf: it links the real C-ABI (header + shared
 * library), exercises a full round-trip, and returns non-zero on any failure.
 * Driven from tests/c_smoke.rs via the system C compiler. */

#include "purrdf.h"

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(cond, msg)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            fprintf(stderr, "C smoke FAILED: %s (line %d)\n", (msg), __LINE__); \
            return 1;                                                           \
        }                                                                       \
    } while (0)

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *stream = fopen(path, "rb");
    if (stream == NULL) {
        return NULL;
    }
    if (fseek(stream, 0, SEEK_END) != 0) {
        fclose(stream);
        return NULL;
    }
    long size = ftell(stream);
    if (size < 0 || fseek(stream, 0, SEEK_SET) != 0) {
        fclose(stream);
        return NULL;
    }
    uint8_t *bytes = malloc((size_t)size);
    if (bytes == NULL || fread(bytes, 1, (size_t)size, stream) != (size_t)size) {
        free(bytes);
        fclose(stream);
        return NULL;
    }
    fclose(stream);
    *length = (size_t)size;
    return bytes;
}

/* A portable substring search over a byte buffer.
 *
 * `memmem` is a GNU extension and this program is compiled `-std=c11`, so the
 * search is spelled out rather than reached for. */
static int contains_bytes(const uint8_t *haystack, size_t haystack_len,
                          const char *needle) {
    size_t needle_len = strlen(needle);
    if (needle_len > haystack_len) {
        return 0;
    }
    for (size_t at = 0; at + needle_len <= haystack_len; at++) {
        if (memcmp(haystack + at, needle, needle_len) == 0) {
            return 1;
        }
    }
    return 0;
}

/* ── The entailment golden vector, walked through the C ABI ──────────────────
 *
 * `crates/validate/tests/fixtures/regime-boundary.vectors` is the ONE artifact
 * the Rust test, the WASM module and the Python suite all check. Reading it here
 * is what makes the "one artifact, four hosts" claim true of the C ABI too: the
 * cases below reach `purrdf_entail_materialize_to_nquads` and both of its
 * out-buffers are compared byte for byte against the committed bodies.
 *
 * The format is line-oriented and deliberately dependency-free (see
 * `parse_regime_vectors` in purrdf-validate): a line starting with '@' is a
 * directive, every other line belongs to the body the last body-directive opened,
 * and outside a body only blank lines and '#' comments are legal. */

typedef struct {
    const char *ptr;
    size_t len;
} Slice;

/* A NUL-terminated copy of `s`; an unopened slice becomes the empty string. */
static char *dup_slice(Slice s) {
    char *out = malloc(s.len + 1);
    if (out == NULL) {
        return NULL;
    }
    if (s.len > 0) {
        memcpy(out, s.ptr, s.len);
    }
    out[s.len] = '\0';
    return out;
}

/* Extend `body` to cover the line `[start, end)`, opening it if it is unopened. */
static void extend(Slice *body, const char *start, const char *end) {
    if (body->ptr == NULL) {
        body->ptr = start;
    }
    body->len = (size_t)(end - body->ptr);
}

/* Run one golden case through the C ABI. Returns 0 on a byte-for-byte match. */
static int run_vector_case(const char *regime, Slice input, Slice program,
                           Slice closure, Slice report) {
    char *input_s = dup_slice(input);
    char *program_s = dup_slice(program);
    char *closure_s = dup_slice(closure);
    char *report_s = dup_slice(report);
    PurrdfBuffer *nquads = NULL;
    PurrdfBuffer *rendered = NULL;
    PurrdfError *error = NULL;
    int failed = 1;

    if (input_s == NULL || program_s == NULL || closure_s == NULL ||
        report_s == NULL) {
        goto done;
    }
    if (purrdf_entail_materialize_to_nquads(input_s, regime, program_s, &nquads,
                                            &rendered,
                                            &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "golden case (%s) did not materialize: %s\n", regime,
                error == NULL ? "(no error)" : purrdf_error_message(error));
        goto done;
    }
    const uint8_t *bytes = NULL;
    size_t len = 0;
    purrdf_buffer_data(nquads, &bytes, &len);
    if (len != strlen(closure_s) || memcmp(bytes, closure_s, len) != 0) {
        fprintf(stderr, "golden case (%s): closure mismatch through the C ABI\n",
                regime);
        goto done;
    }
    purrdf_buffer_data(rendered, &bytes, &len);
    if (len != strlen(report_s) || memcmp(bytes, report_s, len) != 0) {
        fprintf(stderr, "golden case (%s): report mismatch through the C ABI\n",
                regime);
        goto done;
    }
    failed = 0;

done:
    if (error != NULL) {
        purrdf_error_free(error);
    }
    /* Two buffers out, two frees — the ownership contract the header states. */
    if (rendered != NULL) {
        purrdf_buffer_free(rendered);
    }
    if (nquads != NULL) {
        purrdf_buffer_free(nquads);
    }
    free(report_s);
    free(closure_s);
    free(program_s);
    free(input_s);
    return failed;
}

/* Walk every case of the committed artifact. Returns the number of cases run,
 * or -1 on any mismatch or malformed input. */
static int check_golden_vector(const char *path) {
    size_t length = 0;
    uint8_t *raw = read_file(path, &length);
    if (raw == NULL) {
        fprintf(stderr, "cannot read the golden vector at %s\n", path);
        return -1;
    }
    const char *text = (const char *)raw;
    const char *end = text + length;
    char regime[128];
    regime[0] = '\0';
    Slice input = {NULL, 0};
    Slice program = {NULL, 0};
    Slice closure = {NULL, 0};
    Slice report = {NULL, 0};
    int section = 0; /* 0 none, 1 input, 2 closure, 3 report, 4 program */
    int cases = 0;
    int failed = 0;

    for (const char *line = text; line < end;) {
        const char *newline = memchr(line, '\n', (size_t)(end - line));
        const char *stop = newline == NULL ? end : newline + 1;
        if (line[0] != '@') {
            if (section == 1) {
                extend(&input, line, stop);
            } else if (section == 2) {
                extend(&closure, line, stop);
            } else if (section == 3) {
                extend(&report, line, stop);
            } else if (section == 4) {
                extend(&program, line, stop);
            }
            line = stop;
            continue;
        }
        size_t span = (size_t)((newline == NULL ? end : newline) - line);
        section = 0;
        if (span >= 8 && memcmp(line, "@regime ", 8) == 0) {
            size_t n = span - 8;
            if (n >= sizeof regime) {
                n = sizeof regime - 1;
            }
            memcpy(regime, line + 8, n);
            regime[n] = '\0';
        } else if (span == 6 && memcmp(line, "@input", 6) == 0) {
            section = 1;
        } else if (span == 8 && memcmp(line, "@closure", 8) == 0) {
            section = 2;
        } else if (span == 7 && memcmp(line, "@report", 7) == 0) {
            section = 3;
        } else if (span == 8 && memcmp(line, "@program", 8) == 0) {
            section = 4;
        } else if (span == 4 && memcmp(line, "@end", 4) == 0) {
            failed |= run_vector_case(regime, input, program, closure, report);
            cases += 1;
            regime[0] = '\0';
            input.ptr = NULL;
            input.len = 0;
            program.ptr = NULL;
            program.len = 0;
            closure.ptr = NULL;
            closure.len = 0;
            report.ptr = NULL;
            report.len = 0;
        }
        line = stop;
    }
    free(raw);
    return failed != 0 ? -1 : cases;
}

/* ── The caller's `owl:imports` table, from a REAL C caller ──────────────────
 *
 * `webont-imports-011` is the W3C case the parameter exists for: its premise says
 * `Socrates a ont:Man` and `owl:imports <…/support011-A>`, and `Man ⊑ Mortal` lives
 * only in that support document, so the published answer is reachable only from the
 * imports closure. PurRDF fetches nothing, so the document arrives as configuration.
 *
 * This is the C-ABI leg of that claim, and it is load-bearing in a way a Rust test
 * cannot be: the program below is compiled against the COMMITTED `purrdf.h`, so a
 * header that had lost the import parameters would fail to compile here rather than
 * shipping an entry point no C caller could see. */

/* One RDF/XML file as a NUL-terminated N-Quads string, converted through the C ABI.
 *
 * No base IRI is passed and none is needed: every document in the vendored corpus
 * either declares its own `xml:base` or uses only absolute IRIs. The caller frees
 * the result with `free()`. */
static char *rdfxml_file_to_nquads(const char *path) {
    size_t source_len = 0;
    uint8_t *source = read_file(path, &source_len);
    if (source == NULL) {
        return NULL;
    }
    PurrdfDataset *parsed = NULL;
    PurrdfBuffer *serialized = NULL;
    PurrdfError *error = NULL;
    char *out = NULL;
    if (purrdf_parse(source, source_len, "application/rdf+xml", NULL, NULL, &parsed,
                     &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "cannot parse %s: %s\n", path,
                error == NULL ? "(no error)" : purrdf_error_message(error));
        goto done;
    }
    if (purrdf_serialize(parsed, "application/n-quads", NULL, &serialized, NULL,
                         NULL, NULL, &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "cannot serialize %s: %s\n", path,
                error == NULL ? "(no error)" : purrdf_error_message(error));
        goto done;
    }
    const uint8_t *bytes = NULL;
    size_t len = 0;
    purrdf_buffer_data(serialized, &bytes, &len);
    /* The buffer carries bytes and a length, never a NUL, and the entailment
     * entry points take C strings — so the copy is not incidental. */
    Slice slice = {(const char *)bytes, len};
    out = dup_slice(slice);

done:
    if (error != NULL) {
        purrdf_error_free(error);
    }
    if (serialized != NULL) {
        purrdf_buffer_free(serialized);
    }
    if (parsed != NULL) {
        purrdf_dataset_free(parsed);
    }
    free(source);
    return out;
}

/* Drive `webont-imports-011` through the three conclusion-directed services with
 * the vendored support ontology supplied. Returns 0 on success. */
static int check_vendored_imports(const char *premise_path,
                                  const char *conclusion_path,
                                  const char *support_path) {
    char *premise = rdfxml_file_to_nquads(premise_path);
    char *conclusion = rdfxml_file_to_nquads(conclusion_path);
    char *support = rdfxml_file_to_nquads(support_path);
    PurrdfBuffer *answer = NULL;
    PurrdfBuffer *certificate = NULL;
    PurrdfError *error = NULL;
    int failed = 1;

    if (premise == NULL || conclusion == NULL || support == NULL) {
        fprintf(stderr, "the vendored imports case did not convert to N-Quads\n");
        goto done;
    }
    /* The premise really does carry the import, so this cannot pass by having been
     * handed a document that needs none. */
    if (!contains_bytes((const uint8_t *)premise, strlen(premise),
                        "<http://www.w3.org/2002/07/owl#imports>")) {
        fprintf(stderr, "the vendored premise carries no owl:imports\n");
        goto done;
    }

    /* The ontology IRI the support document DECLARES — the name the premise's
     * `owl:imports` object actually is, not the file it happens to live in. */
    const char *import_iris[1] = {"http://www.w3.org/2002/03owlt/imports/support011-A"};
    const char *import_documents[1] = {support};

    const uint8_t *bytes = NULL;
    size_t len = 0;

    if (purrdf_entail_graph_entails("owl-rl", premise, conclusion, import_iris,
                                    import_documents, 1, &answer, &certificate,
                                    &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "graph_entails refused the vendored case: %s\n",
                error == NULL ? "(no error)" : purrdf_error_message(error));
        goto done;
    }
    purrdf_buffer_data(answer, &bytes, &len);
    if (!contains_bytes(bytes, len, "entailment entailed\n")) {
        fprintf(stderr, "the vendored imports case did not answer entailed\n");
        goto done;
    }
    purrdf_buffer_data(certificate, &bytes, &len);
    if (!contains_bytes(bytes, len, "purrdf-reasoning-report 4\n")) {
        fprintf(stderr, "the vendored imports case carried no report\n");
        goto done;
    }
    purrdf_buffer_free(answer);
    purrdf_buffer_free(certificate);
    answer = NULL;
    certificate = NULL;

    /* The pattern-shaped entry point answers the same question the same way: a
     * conclusion graph is the relation with no columns, so a `yes` is one bare row. */
    if (purrdf_entail_certain_answers("owl-rl", premise, conclusion, import_iris,
                                      import_documents, 1, &answer, &certificate,
                                      &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "certain_answers refused the vendored case\n");
        goto done;
    }
    purrdf_buffer_data(answer, &bytes, &len);
    if (len != strlen("mechanism strict-table\nrow\n") ||
        memcmp(bytes, "mechanism strict-table\nrow\n", len) != 0) {
        fprintf(stderr, "the vendored imports case is not one bare row\n");
        goto done;
    }
    purrdf_buffer_free(answer);
    purrdf_buffer_free(certificate);
    answer = NULL;
    certificate = NULL;

    if (purrdf_entail_verify_entailment("owl-rl", premise, conclusion, import_iris,
                                        import_documents, 1, &answer, &certificate,
                                        &error) != PURRDF_STATUS_OK) {
        fprintf(stderr, "verify_entailment refused the vendored case\n");
        goto done;
    }
    purrdf_buffer_data(answer, &bytes, &len);
    if (!contains_bytes(bytes, len, "warrant present\nverified true\n")) {
        fprintf(stderr, "the vendored imports warrant did not re-decide\n");
        goto done;
    }
    purrdf_buffer_free(answer);
    purrdf_buffer_free(certificate);
    answer = NULL;
    certificate = NULL;

    /* An empty table with two NULL arrays is accepted as "imports nothing" — and
     * for THIS premise that is a refusal NAMING the document, never an answer
     * computed from a premise missing the axioms it told the caller about. */
    if (purrdf_entail_graph_entails("owl-rl", premise, conclusion, NULL, NULL, 0,
                                    &answer, &certificate,
                                    &error) != PURRDF_STATUS_PARSE_ERROR) {
        fprintf(stderr, "an unsupplied import was not refused\n");
        goto done;
    }
    if (answer != NULL || certificate != NULL) {
        fprintf(stderr, "a refused call handed out a buffer to free\n");
        goto done;
    }
    if (error == NULL ||
        !contains_bytes((const uint8_t *)purrdf_error_message(error),
                        strlen(purrdf_error_message(error)),
                        "http://www.w3.org/2002/03owlt/imports/support011-A")) {
        fprintf(stderr, "the refusal did not name the unresolved document\n");
        goto done;
    }
    purrdf_error_free(error);
    error = NULL;

    /* A NULL array with a NON-ZERO count is a caller error, refused before any
     * dereference rather than segfaulting. */
    if (purrdf_entail_graph_entails("owl-rl", premise, conclusion, NULL, NULL, 1,
                                    &answer, &certificate,
                                    &error) != PURRDF_STATUS_NULL_POINTER) {
        fprintf(stderr, "a null import array with a non-zero count was not refused\n");
        goto done;
    }
    if (error != NULL) {
        purrdf_error_free(error);
        error = NULL;
    }

    failed = 0;

done:
    if (error != NULL) {
        purrdf_error_free(error);
    }
    if (answer != NULL) {
        purrdf_buffer_free(answer);
    }
    if (certificate != NULL) {
        purrdf_buffer_free(certificate);
    }
    free(premise);
    free(conclusion);
    free(support);
    return failed;
}

int main(int argc, char **argv) {
    CHECK(argc == 7,
          "shared OKF fixture, OKF config, entailment golden vector, and the three "
          "vendored webont-imports-011 documents");
    /* ABI version */
    PurrdfAbiVersion version;
    CHECK(purrdf_abi_version(&version) == PURRDF_STATUS_OK, "abi_version");
    printf("libpurrdf ABI %u.%u.%u\n", version.major, version.minor, version.patch);
    /* The invariant a real C consumer depends on: the library it LINKED against
     * reports the same ABI the header it COMPILED against declares. Comparing
     * against the header macros rather than a literal means an intentional bump
     * needs no edit here, while a library/header mismatch — the exact condition
     * that silently mis-binds arguments — still fails loudly. The prototype list
     * behind this triple is frozen in tests/abi_signatures.snapshot, and the
     * literal `0.7.0` is pinned in tests/abi.rs. */
    CHECK(version.major == PURRDF_ABI_MAJOR && version.minor == PURRDF_ABI_MINOR &&
              version.patch == PURRDF_ABI_PATCH,
          "linked library reports the header's ABI version");

    /* parse */
    const char *doc = "<http://a> <http://b> <http://c> .";
    PurrdfDataset *dataset = NULL;
    PurrdfError *error = NULL;
    int rc = purrdf_parse((const uint8_t *)doc, strlen(doc), "text/turtle", NULL,
                          NULL, &dataset, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && dataset != NULL, "parse");

    size_t quad_count = 0;
    CHECK(purrdf_dataset_quad_count(dataset, &quad_count) == PURRDF_STATUS_OK,
          "quad_count");
    CHECK(quad_count == 1, "one quad");

    /* capabilities */
    PurrdfCapabilities caps;
    CHECK(purrdf_capabilities(dataset, &caps) == PURRDF_STATUS_OK, "capabilities");
    CHECK(caps.quoted_triples == 0, "plain graph has no star layer");

    /* pattern cursor */
    PurrdfGraphMatch any;
    memset(&any, 0, sizeof(any));
    any.kind = PURRDF_GRAPH_MATCH_KIND_ANY;
    PurrdfCursor *cursor = NULL;
    rc = purrdf_quads_for_pattern(dataset, NULL, NULL, NULL, &any, &cursor, &error);
    CHECK(rc == PURRDF_STATUS_OK && cursor != NULL, "quads_for_pattern");

    int rows = 0;
    PurrdfTermView s, p, o, g;
    uint8_t has_graph = 0;
    while ((rc = purrdf_cursor_next(cursor, &s, &p, &o, &g, &has_graph)) ==
           PURRDF_STATUS_OK) {
        printf("  quad: subject=%.*s\n", (int)s.lexical.len, (const char *)s.lexical.ptr);
        CHECK(s.kind == PURRDF_TERM_KIND_IRI, "subject is an IRI");
        rows++;
    }
    CHECK(rc == PURRDF_STATUS_CURSOR_EXHAUSTED, "cursor exhausted");
    CHECK(rows == 1, "one row iterated");
    purrdf_cursor_free(cursor);

    /* serialize */
    PurrdfBuffer *serialized = NULL;
    size_t dropped = 99;
    size_t directional = 99;
    size_t named_graph_rows = 99;
    rc = purrdf_serialize(dataset, "application/n-triples", NULL, &serialized,
                          &dropped, &directional, &named_graph_rows, &error);
    CHECK(rc == PURRDF_STATUS_OK && serialized != NULL, "serialize");
    CHECK(dropped == 0, "no statement rows dropped for n-triples");
    CHECK(directional == 0, "no base directions dropped for n-triples");
    CHECK(named_graph_rows == 0,
          "no named-graph rows dropped: this dataset has no named graph");
    const uint8_t *sbytes = NULL;
    size_t slen = 0;
    CHECK(purrdf_buffer_data(serialized, &sbytes, &slen) == PURRDF_STATUS_OK,
          "buffer_data");
    CHECK(slen > 0, "serialized bytes present");
    purrdf_buffer_free(serialized);

    /* A graph-carrying dataset meeting a single-graph syntax.
     *
     * This lane FLATTENS and COUNTS: it does not refuse the way the query lane
     * does, and it does not widen to a syntax the caller did not name. Both rows
     * below are scoped to a named graph, so `text/turtle` is the correct rendering
     * of an EMPTY default graph — status OK, no error object, zero bytes — with the
     * whole of the loss in `out_named_graph_rows_dropped`. Asserted from C because
     * that is the contract purrdf.h publishes to a C consumer, and prose alone has
     * already drifted from it once. */
    const char *graph_doc = "<http://s1> <http://p> <http://o1> <http://g1> .\n"
                            "<http://s2> <http://p> <http://o2> <http://g2> .\n";
    PurrdfDataset *graph_dataset = NULL;
    rc = purrdf_parse((const uint8_t *)graph_doc, strlen(graph_doc),
                      "application/n-quads", NULL, NULL, &graph_dataset, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && graph_dataset != NULL,
          "parse an all-named-graph n-quads document");

    PurrdfBuffer *flattened = NULL;
    size_t flat_statements = 99;
    size_t flat_directional = 99;
    size_t flat_named_graph_rows = 99;
    rc = purrdf_serialize(graph_dataset, "text/turtle", NULL, &flattened,
                          &flat_statements, &flat_directional,
                          &flat_named_graph_rows, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && flattened != NULL,
          "turtle serialization of a graph-carrying dataset SUCCEEDS (no refusal)");
    CHECK(flat_statements == 0, "turtle carries the star layer: nothing charged there");
    CHECK(flat_directional == 0, "no directional literal in this document");
    CHECK(flat_named_graph_rows == 2,
          "both graph-scoped base quads charged to the named-graph count");
    const uint8_t *fbytes = NULL;
    size_t flen = 99;
    CHECK(purrdf_buffer_data(flattened, &fbytes, &flen) == PURRDF_STATUS_OK,
          "buffer_data over the flattened document");
    CHECK(flen == 0, "an all-named-graph dataset flattens to an EMPTY turtle document");
    purrdf_buffer_free(flattened);

    /* The same call with every count DECLINED. Null means "do not report", never
     * "do not serialize": identical status, identical (empty) bytes, no error. The
     * counts are the only signal this lane offers, so a caller that declines all
     * three has asked for the document alone and receives exactly that. */
    PurrdfBuffer *flattened_unreported = NULL;
    rc = purrdf_serialize(graph_dataset, "text/turtle", NULL, &flattened_unreported,
                          NULL, NULL, NULL, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && flattened_unreported != NULL,
          "null loss counts still serialize");
    const uint8_t *ubytes = NULL;
    size_t ulen = 99;
    CHECK(purrdf_buffer_data(flattened_unreported, &ubytes, &ulen) ==
              PURRDF_STATUS_OK,
          "buffer_data with the counts declined");
    CHECK(ulen == 0, "declining the counts does not change the document");
    purrdf_buffer_free(flattened_unreported);

    /* And the loss is a property of the TARGET, not of the dataset: a
     * dataset-capable syntax keeps both graphs and charges nothing. */
    PurrdfBuffer *kept = NULL;
    flat_named_graph_rows = 99;
    rc = purrdf_serialize(graph_dataset, "application/n-quads", NULL, &kept,
                          &flat_statements, &flat_directional,
                          &flat_named_graph_rows, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && kept != NULL,
          "n-quads serialization of the same dataset");
    CHECK(flat_named_graph_rows == 0, "a dataset-capable target drops no graph row");
    const uint8_t *kbytes = NULL;
    size_t klen = 0;
    CHECK(purrdf_buffer_data(kept, &kbytes, &klen) == PURRDF_STATUS_OK,
          "buffer_data over the n-quads document");
    CHECK(contains_bytes(kbytes, klen, "http://g1") &&
              contains_bytes(kbytes, klen, "http://g2"),
          "both graph names survive into n-quads");
    purrdf_buffer_free(kept);
    purrdf_dataset_free(graph_dataset);

    /* GTS round-trip (plain graph) */
    PurrdfBuffer *gts = NULL;
    rc = purrdf_to_gts(dataset, "dist", &gts, &error);
    CHECK(rc == PURRDF_STATUS_OK && gts != NULL, "to_gts");
    const uint8_t *gbytes = NULL;
    size_t glen = 0;
    purrdf_buffer_data(gts, &gbytes, &glen);
    CHECK(glen > 0, "gts bytes present");
    PurrdfDataset *restored = NULL;
    rc = purrdf_from_gts(gbytes, glen, &restored, &error);
    CHECK(rc == PURRDF_STATUS_OK && restored != NULL, "from_gts");
    size_t restored_count = 0;
    purrdf_dataset_quad_count(restored, &restored_count);
    CHECK(restored_count == 1, "gts round-trip preserves the quad");
    purrdf_buffer_free(gts);
    purrdf_dataset_free(restored);

    /* deterministic graph/tabular/research-object carrier surface + explicit ledger */
    const char *projection_config =
        "{\"profile\":\"lpg-csv\",\"config\":{\"rdf_type\":"
        "\"https://example.org/type\",\"scope\":{\"mode\":\"all\"},"
        "\"limits\":{\"max_artifacts\":16,"
        "\"max_artifact_bytes\":1000000,\"max_total_bytes\":4000000,"
        "\"max_archive_bytes\":5000000,\"max_term_depth\":16},"
        "\"execution_limits\":{\"max_input_records\":1000,"
        "\"max_model_records\":1000,\"max_nodes\":1000,"
        "\"max_edges\":1000}}}";
    PurrdfBuffer *projection = NULL;
    PurrdfBuffer *project_ledger = NULL;
    rc = purrdf_project(dataset, "lpg-csv",
                        (const uint8_t *)projection_config,
                        strlen(projection_config), &projection, &project_ledger,
                        &error);
    CHECK(rc == PURRDF_STATUS_OK && projection != NULL && project_ledger != NULL,
          "project");
    const uint8_t *projection_bytes = NULL;
    size_t projection_len = 0;
    purrdf_buffer_data(projection, &projection_bytes, &projection_len);
    CHECK(projection_len > 0, "projection archive bytes present");
    const uint8_t *ledger_bytes = NULL;
    size_t ledger_len = 0;
    purrdf_buffer_data(project_ledger, &ledger_bytes, &ledger_len);
    const char *ledger_prefix = "{\n  \"schema_version\": 1,";
    CHECK(ledger_len >= strlen(ledger_prefix) &&
              memcmp(ledger_bytes, ledger_prefix, strlen(ledger_prefix)) == 0,
          "projection ledger JSON present");
    PurrdfDataset *projection_restored = NULL;
    PurrdfBuffer *lift_ledger = NULL;
    rc = purrdf_lift(projection_bytes, projection_len, "lpg-csv",
                     (const uint8_t *)projection_config, strlen(projection_config),
                     &projection_restored, &lift_ledger, &error);
    CHECK(rc == PURRDF_STATUS_OK && projection_restored != NULL && lift_ledger != NULL,
          "lift");
    size_t projection_restored_count = 0;
    purrdf_dataset_quad_count(projection_restored, &projection_restored_count);
    CHECK(projection_restored_count == 1, "projection round-trip preserves the quad");
    purrdf_buffer_free(lift_ledger);
    purrdf_dataset_free(projection_restored);
    purrdf_buffer_free(project_ledger);
    purrdf_buffer_free(projection);

    /* caller-declared curated CSVW terms is projected through the same C ABI */
    const char *terms_config =
        "{\"profile\":\"csvw-terms\",\"config\":{"
        "\"csvw\":{\"metadata_base_iri\":\"https://example.org/catalog/metadata.json\","
        "\"context\":{\"iri\":\"http://www.w3.org/ns/csvw\",\"prefixes\":{}},"
        "\"table_group_iri\":\"https://example.org/catalog\","
        "\"vocabulary\":{\"csvw_namespace\":\"http://www.w3.org/ns/csvw#\","
        "\"rdf_namespace\":\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\","
        "\"rdfs_namespace\":\"http://www.w3.org/2000/01/rdf-schema#\","
        "\"xsd_namespace\":\"http://www.w3.org/2001/XMLSchema#\"},"
        "\"mode\":\"minimal\",\"limits\":{\"max_artifacts\":8,"
        "\"max_artifact_bytes\":1000000,\"max_total_bytes\":4000000,"
        "\"max_archive_bytes\":5000000,\"max_term_depth\":16},\"max_records\":1000},"
        "\"metadata_path\":\"csvw-metadata.json\","
        "\"graph_selection\":{\"kind\":\"all\"},\"tables\":[{"
        "\"name\":\"terms\",\"table_url\":\"https://example.org/catalog/terms.csv\","
        "\"artifact_path\":\"terms.csv\",\"selector\":{\"type_predicate\":null,"
        "\"any_types\":[],\"all_types\":[],\"none_types\":[],"
        "\"iri_prefixes\":[\"https://example.org/\"]},\"identity\":{"
        "\"name\":\"iri\",\"titles\":{},\"datatype\":{\"id\":null,"
        "\"base\":\"http://www.w3.org/2001/XMLSchema#anyURI\",\"format\":null,"
        "\"length\":null,\"min_length\":null,\"max_length\":null,"
        "\"minimum\":null,\"maximum\":null,\"min_inclusive\":null,"
        "\"max_inclusive\":null,\"min_exclusive\":null,\"max_exclusive\":null}},"
        "\"columns\":[{\"name\":\"object\",\"titles\":{},"
        "\"predicate\":\"https://example.org/p\",\"value_mode\":{\"kind\":\"iri\","
        "\"datatype\":{\"id\":null,\"base\":\"http://www.w3.org/2001/XMLSchema#anyURI\","
        "\"format\":null,\"length\":null,\"min_length\":null,\"max_length\":null,"
        "\"minimum\":null,\"maximum\":null,\"min_inclusive\":null,"
        "\"max_inclusive\":null,\"min_exclusive\":null,\"max_exclusive\":null}},"
        "\"cardinality\":{\"kind\":\"one\"},\"required\":false}]}],"
        "\"execution_limits\":{\"max_rows\":100,\"max_values\":1000,"
        "\"max_values_per_cell\":10}}}";
    PurrdfBuffer *terms_projection = NULL;
    PurrdfBuffer *terms_ledger = NULL;
    rc = purrdf_project(dataset, "csvw-terms", (const uint8_t *)terms_config,
                        strlen(terms_config), &terms_projection, &terms_ledger,
                        &error);
    CHECK(rc == PURRDF_STATUS_OK && terms_projection != NULL && terms_ledger != NULL,
          "project csvw-terms");
    purrdf_buffer_free(terms_ledger);
    purrdf_buffer_free(terms_projection);

    /* the shared strict OKF terms fixture reaches the exact same Rust engine */
    size_t okf_source_len = 0;
    size_t okf_config_len = 0;
    uint8_t *okf_source = read_file(argv[1], &okf_source_len);
    uint8_t *okf_config = read_file(argv[2], &okf_config_len);
    CHECK(okf_source != NULL && okf_config != NULL, "read shared OKF fixtures");
    PurrdfDataset *okf_dataset = NULL;
    rc = purrdf_parse(okf_source, okf_source_len, "application/trig", NULL, NULL,
                      &okf_dataset, &error);
    CHECK(rc == PURRDF_STATUS_OK && okf_dataset != NULL, "parse shared OKF source");
    PurrdfBuffer *okf_projection = NULL;
    PurrdfBuffer *okf_ledger = NULL;
    rc = purrdf_project(okf_dataset, "okf-terms", okf_config, okf_config_len,
                        &okf_projection, &okf_ledger, &error);
    CHECK(rc == PURRDF_STATUS_OK && okf_projection != NULL && okf_ledger != NULL,
          "project shared OKF terms fixture");
    const uint8_t *okf_projection_bytes = NULL;
    size_t okf_projection_len = 0;
    purrdf_buffer_data(okf_projection, &okf_projection_bytes, &okf_projection_len);
    CHECK(okf_projection_bytes != NULL && okf_projection_len == 6144,
          "shared OKF archive has exact canonical size");
    const uint8_t *okf_ledger_bytes = NULL;
    size_t okf_ledger_len = 0;
    purrdf_buffer_data(okf_ledger, &okf_ledger_bytes, &okf_ledger_len);
    CHECK(okf_ledger_bytes != NULL && okf_ledger_len > 0,
          "shared OKF projection carries its loss ledger");
    purrdf_buffer_free(okf_ledger);
    purrdf_buffer_free(okf_projection);
    purrdf_dataset_free(okf_dataset);
    free(okf_config);
    free(okf_source);

    /* SPARQL JSON */
    PurrdfBuffer *json = NULL;
    rc = purrdf_query_json(dataset, "SELECT ?s WHERE { ?s ?p ?o }", NULL, NULL,
                           NULL, &json, &error);
    CHECK(rc == PURRDF_STATUS_OK && json != NULL, "query_json");
    const uint8_t *jbytes = NULL;
    size_t jlen = 0;
    purrdf_buffer_data(json, &jbytes, &jlen);
    CHECK(jlen > 0, "sparql-json bytes present");
    purrdf_buffer_free(json);

    /* `provenance_prefix`/`provenance_iri` anchor the additive purrdf extension. */
    PurrdfBuffer *prov_json = NULL;
    rc = purrdf_query_json(dataset, "SELECT ?s WHERE { ?s ?p ?o }", NULL, "prov",
                           "https://example.org/ns/prov#", &prov_json, &error);
    CHECK(rc == PURRDF_STATUS_OK && prov_json != NULL, "query_json with provenance");
    const uint8_t *pbytes = NULL;
    size_t plen = 0;
    purrdf_buffer_data(prov_json, &pbytes, &plen);
    CHECK(contains_bytes(pbytes, plen, "\"prov\":{"),
          "additive prov member present");
    CHECK(contains_bytes(pbytes, plen, "\"engine\":\"purrdf-sparql-eval\""),
          "engine populated");
    purrdf_buffer_free(prov_json);

    /* governed SPARQL: exhaustion is an OK outcome carrying typed evidence and a
     * certified result, never a query error or a complete answer. */
    PurrdfQueryGovernors governors;
    CHECK(purrdf_query_governors_init(&governors) == PURRDF_STATUS_OK,
          "governor initializer");
    governors.enabled = PURRDF_GOVERNOR_FLAG_MAX_ANSWERS;
    governors.max_answers = 0;
    int32_t query_outcome = -1;
    int32_t result_kind = -1;
    PurrdfRowCursor *partial_rows = NULL;
    PurrdfDataset *partial_graph = NULL;
    uint8_t partial_boolean = 0;
    PurrdfGovernorEvidence query_evidence;
    PurrdfPartialCertificate partial_certificate;
    rc = purrdf_query_governed(
        dataset, "SELECT ?s WHERE { ?s ?p ?o }", NULL, NULL, &governors,
        &query_outcome, &result_kind, &partial_rows, &partial_graph,
        &partial_boolean, &query_evidence, &partial_certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL, "governed query outcome");
    CHECK(query_outcome == PURRDF_QUERY_OUTCOME_KIND_BUDGET_EXHAUSTED,
          "governed query is typed exhaustion");
    CHECK(result_kind == PURRDF_RESULT_KIND_SOLUTIONS,
          "governed SELECT names its result kind");
    CHECK(query_evidence.trip.kind == PURRDF_GOVERNOR_TRIP_KIND_BUDGET &&
              query_evidence.trip.dimension ==
                  PURRDF_RESOURCE_DIMENSION_ANSWER_ROWS,
          "governed query carries answer-cap evidence");
    CHECK(partial_certificate.kind == PURRDF_PARTIAL_KIND_CERTAIN,
          "governed query carries a certain lower bound");
    if (partial_rows != NULL) {
        purrdf_rowcursor_free(partial_rows);
    }
    CHECK(partial_graph == NULL, "a SELECT writes no partial graph");

    /* `aggregate_namespace` end-to-end: registers purrdf's first-party statistical
     * aggregate set and actually COMPUTES `MEDIAN` through the real C ABI (header +
     * linkage) — the reachability gap this parameter closes. */
    const char *median_doc =
        "<http://example.org/s1> <http://example.org/value> \"1\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n"
        "<http://example.org/s2> <http://example.org/value> \"2\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n"
        "<http://example.org/s3> <http://example.org/value> \"3\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n";
    PurrdfDataset *median_dataset = NULL;
    rc = purrdf_parse((const uint8_t *)median_doc, strlen(median_doc), "text/turtle",
                      NULL, NULL, &median_dataset, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL && median_dataset != NULL,
          "median fixture parses");

    CHECK(purrdf_query_governors_init(&governors) == PURRDF_STATUS_OK,
          "median governor initializer");
    int32_t median_outcome = -1;
    int32_t median_kind = -1;
    PurrdfRowCursor *median_rows = NULL;
    PurrdfGovernorEvidence median_evidence;
    PurrdfPartialCertificate median_partial;
    rc = purrdf_query_governed(
        median_dataset,
        "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) "
        "WHERE { ?s <http://example.org/value> ?v }",
        NULL, "https://example.org/agg#", &governors, &median_outcome,
        &median_kind, &median_rows, NULL, NULL, &median_evidence,
        &median_partial, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL, "aggregate_namespace query runs");
    CHECK(median_outcome == PURRDF_QUERY_OUTCOME_KIND_COMPLETE,
          "aggregate_namespace query completes");
    CHECK(purrdf_rowcursor_next(median_rows) == PURRDF_STATUS_OK,
          "MEDIAN row present");
    PurrdfTermView median_view;
    uint8_t median_bound = 0;
    CHECK(purrdf_rowcursor_term(median_rows, 0, &median_view, &median_bound) ==
                  PURRDF_STATUS_OK &&
              median_bound == 1,
          "?m is bound");
    CHECK(median_view.lexical.len == 1 && median_view.lexical.ptr[0] == '2',
          "MEDIAN of {1, 2, 3} is 2");
    purrdf_rowcursor_free(median_rows);
    purrdf_dataset_free(median_dataset);

    /* The entailment-aware carrier keeps phase two and its closure report together. */
    CHECK(purrdf_query_governors_init(&governors) == PURRDF_STATUS_OK,
          "entailment governor initializer");
    int32_t entailment_outcome = -1;
    int32_t entailment_kind = -1;
    uint8_t entailment_boolean = 0;
    PurrdfGovernedEntailmentEvidence entailment_evidence;
    PurrdfPartialCertificate entailment_partial;
    PurrdfBuffer *entailment_report = NULL;
    rc = purrdf_query_entailment_governed(
        dataset, "ASK { ?s ?p ?o }", NULL, "simple", "", NULL, &governors,
        &entailment_outcome, &entailment_kind, NULL, NULL,
        &entailment_boolean, &entailment_evidence, &entailment_partial,
        &entailment_report, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL,
          "governed entailment query outcome");
    CHECK(entailment_outcome == PURRDF_ENTAILMENT_QUERY_OUTCOME_KIND_COMPLETE &&
              entailment_kind == PURRDF_RESULT_KIND_BOOLEAN &&
              entailment_boolean == 1,
          "governed entailment query answers");
    CHECK(entailment_evidence.query_ran == 1 && entailment_report != NULL,
          "governed entailment query carries both phases");
    purrdf_buffer_free(entailment_report);

    /* `aggregate_namespace` reaches MEDIAN over a binding the RDFS closure itself
     * produced: three cats entailed `Animal`, each carrying a distinct weight. */
    const char *entailed_median_doc =
        "<http://example.org/Cat> "
        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> "
        "<http://example.org/Animal> .\n"
        "<http://example.org/tom> "
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        "<http://example.org/Cat> .\n"
        "<http://example.org/tom> <http://example.org/weight> \"1\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n"
        "<http://example.org/felix> "
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        "<http://example.org/Cat> .\n"
        "<http://example.org/felix> <http://example.org/weight> \"2\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n"
        "<http://example.org/garfield> "
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        "<http://example.org/Cat> .\n"
        "<http://example.org/garfield> <http://example.org/weight> \"3\"^^"
        "<http://www.w3.org/2001/XMLSchema#integer> .\n";
    PurrdfDataset *entailed_median_dataset = NULL;
    rc = purrdf_parse((const uint8_t *)entailed_median_doc,
                      strlen(entailed_median_doc), "text/turtle", NULL, NULL,
                      &entailed_median_dataset, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL &&
              entailed_median_dataset != NULL,
          "entailed median fixture parses");
    CHECK(purrdf_query_governors_init(&governors) == PURRDF_STATUS_OK,
          "entailed median governor initializer");
    int32_t entailed_median_outcome = -1;
    int32_t entailed_median_kind = -1;
    PurrdfRowCursor *entailed_median_rows = NULL;
    PurrdfGovernedEntailmentEvidence entailed_median_evidence;
    PurrdfPartialCertificate entailed_median_partial;
    PurrdfBuffer *entailed_median_report = NULL;
    rc = purrdf_query_entailment_governed(
        entailed_median_dataset,
        "PREFIX ex: <http://example.org/> "
        "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?w) AS ?m) "
        "WHERE { ?s a ex:Animal . ?s ex:weight ?w }",
        NULL, "rdfs", "", "https://example.org/agg#", &governors,
        &entailed_median_outcome, &entailed_median_kind,
        &entailed_median_rows, NULL, NULL, &entailed_median_evidence,
        &entailed_median_partial, &entailed_median_report, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL,
          "aggregate_namespace query over an entailed closure runs");
    CHECK(entailed_median_outcome ==
              PURRDF_ENTAILMENT_QUERY_OUTCOME_KIND_COMPLETE,
          "aggregate_namespace query over an entailed closure completes");
    CHECK(purrdf_rowcursor_next(entailed_median_rows) == PURRDF_STATUS_OK,
          "entailed MEDIAN row present");
    PurrdfTermView entailed_median_view;
    uint8_t entailed_median_bound = 0;
    CHECK(purrdf_rowcursor_term(entailed_median_rows, 0, &entailed_median_view,
                                &entailed_median_bound) == PURRDF_STATUS_OK &&
              entailed_median_bound == 1,
          "entailed ?m is bound");
    CHECK(entailed_median_view.lexical.len == 1 &&
              entailed_median_view.lexical.ptr[0] == '2',
          "MEDIAN of the entailed {1, 2, 3} is 2");
    purrdf_buffer_free(entailed_median_report);
    purrdf_rowcursor_free(entailed_median_rows);
    purrdf_dataset_free(entailed_median_dataset);

    /* C cancellation is a shareable monotone handle. */
    PurrdfCancellation *cancellation = NULL;
    uint8_t cancelled = 0;
    CHECK(purrdf_cancellation_new(&cancellation) == PURRDF_STATUS_OK &&
              cancellation != NULL,
          "cancellation new");
    CHECK(purrdf_cancellation_cancel(cancellation) == PURRDF_STATUS_OK,
          "cancellation cancel");
    CHECK(purrdf_cancellation_is_cancelled(cancellation, &cancelled) ==
                  PURRDF_STATUS_OK &&
              cancelled == 1,
          "cancellation latch");
    purrdf_cancellation_free(cancellation);

    /* Governed UPDATE publishes all or nothing. Zero fuel must preserve the same
     * dataset contents and report exhaustion through the shared evidence carrier. */
    CHECK(purrdf_query_governors_init(&governors) == PURRDF_STATUS_OK,
          "update governor initializer");
    governors.enabled = PURRDF_GOVERNOR_FLAG_FUEL;
    governors.fuel = 0;
    int32_t update_outcome = -1;
    PurrdfGovernorEvidence update_evidence;
    rc = purrdf_update_governed(
        dataset,
        "INSERT DATA { <http://new> <http://predicate> <http://value> }", NULL,
        NULL, &governors, &update_outcome, &update_evidence, &error);
    CHECK(rc == PURRDF_STATUS_OK && error == NULL, "governed update outcome");
    CHECK(update_outcome == PURRDF_UPDATE_OUTCOME_KIND_BUDGET_EXHAUSTED,
          "governed update is typed exhaustion");
    CHECK(update_evidence.trip.dimension == PURRDF_RESOURCE_DIMENSION_FUEL,
          "governed update carries fuel evidence");
    size_t after_update_count = 0;
    CHECK(purrdf_dataset_quad_count(dataset, &after_update_count) ==
                  PURRDF_STATUS_OK &&
              after_update_count == 1,
          "exhausted update applied no mutation");

    /* error path: malformed input produces a readable error, no abort */
    const char *bad = "<http://a> <http://b> @@@";
    PurrdfDataset *bad_dataset = NULL;
    PurrdfError *bad_error = NULL;
    rc = purrdf_parse((const uint8_t *)bad, strlen(bad), "text/turtle", NULL, NULL,
                      &bad_dataset, &bad_error);
    CHECK(rc == PURRDF_STATUS_PARSE_ERROR, "malformed parse error");
    CHECK(bad_dataset == NULL && bad_error != NULL, "error set");
    CHECK(purrdf_error_message(bad_error) != NULL, "error message present");
    purrdf_error_free(bad_error);

    /* ── entailment: the tri-host golden vector, and the reasoning services ── */
    int golden_cases = check_golden_vector(argv[3]);
    CHECK(golden_cases > 0, "the committed entailment golden vector runs through the C ABI");
    printf("entailment golden vector: %d case(s) matched through the C ABI\n",
           golden_cases);

    /* the rule inventories: the specification's counts, which do not move */
    PurrdfBuffer *rule_table = NULL;
    rc = purrdf_entail_rules("owl-rl", &rule_table, &error);
    CHECK(rc == PURRDF_STATUS_OK && rule_table != NULL, "entail_rules(owl-rl)");
    const uint8_t *rule_bytes = NULL;
    size_t rule_len = 0;
    purrdf_buffer_data(rule_table, &rule_bytes, &rule_len);
    CHECK(rule_len > 0, "the OWL 2 RL rule table is not empty");
    purrdf_buffer_free(rule_table);

    /* what this build fires BEYOND that table — a third, disjoint inventory. The
     * exact bytes, not merely non-emptiness: a caller filtering for strictly
     * normative conclusions has to be able to read the name. */
    PurrdfBuffer *added = NULL;
    /* The conclusion-directed services, with the caller's `owl:imports` table —
       the parameter that makes a premise naming another ontology answerable here
       at all. */
    CHECK(check_vendored_imports(argv[4], argv[5], argv[6]) == 0,
          "webont-imports-011 answers from its own premise, owl:imports intact");
    printf("webont-imports-011: entailed through the C ABI with its support "
           "ontology supplied\n");

    rc = purrdf_entail_extensions("owl-rl", &added, &error);
    CHECK(rc == PURRDF_STATUS_OK && added != NULL, "entail_extensions(owl-rl)");
    const uint8_t *added_bytes = NULL;
    size_t added_len = 0;
    purrdf_buffer_data(added, &added_bytes, &added_len);
    CHECK(added_len == strlen("ext-eq-diff-sym\n") &&
              memcmp(added_bytes, "ext-eq-diff-sym\n", added_len) == 0,
          "the OWL 2 RL lane's one extension is named");
    printf("entail_extensions(owl-rl): %.*s", (int)added_len, (const char *)added_bytes);
    purrdf_buffer_free(added);

    /* and a lane with nothing added to it is EMPTY, not absent */
    PurrdfBuffer *none = NULL;
    rc = purrdf_entail_extensions("rdfs", &none, &error);
    CHECK(rc == PURRDF_STATUS_OK && none != NULL, "entail_extensions(rdfs)");
    const uint8_t *none_bytes = NULL;
    size_t none_len = 0;
    purrdf_buffer_data(none, &none_bytes, &none_len);
    CHECK(none_len == 0, "RDFS has had no extension taken for it");
    purrdf_buffer_free(none);

    /* the OWL 2 Direct-Semantics services: an answer AND its certificate.
     * `A ⊑ B ⊑ C` with one instance of `A` — enough to entail `A ⊑ C`, which is
     * asserted nowhere. */
    const char *taxonomy =
        "<http://example.org/A> "
        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .\n"
        "<http://example.org/B> "
        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n"
        "<http://example.org/x> "
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
    const char *chain_axiom =
        "<http://example.org/A> "
        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n";

    PurrdfBuffer *answer = NULL;
    PurrdfBuffer *certificate = NULL;
    const uint8_t *abytes = NULL;
    size_t alen = 0;
    const uint8_t *cbytes = NULL;
    size_t clen = 0;

    rc = purrdf_entail_consistency(taxonomy, 0, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK && answer != NULL && certificate != NULL,
          "entail_consistency");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen == strlen("consistency true\n") &&
              memcmp(abytes, "consistency true\n", alen) == 0,
          "the taxonomy has a model");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(clen > 0 && memcmp(cbytes, "purrdf-dl-certificate 1\n",
                             strlen("purrdf-dl-certificate 1\n")) == 0,
          "the DL certificate is not the chase report");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_entails(taxonomy, chain_axiom, 0, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_entails");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen > strlen("entails true\n") &&
              memcmp(abytes, "entails true\n", strlen("entails true\n")) == 0,
          "the chain entails A subClassOf C");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    /* a narrowed step cap answers `unknown`, never `false`: the third
     * completeness state, reachable through the C ABI. */
    rc = purrdf_entail_entails(taxonomy, chain_axiom, 1, 0, &answer, &certificate,
                               &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_entails under a narrowed cap");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen > strlen("entails unknown\n") &&
              memcmp(abytes, "entails unknown\n", strlen("entails unknown\n")) == 0,
          "an exhausted search is unknown, not false");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "completeness budget-exhausted\n"),
          "the certificate says the budget ran out");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    /* the SECOND cap, narrowed the same way. A round is a pass rather than a
     * unit of cost, so this one bounds the work done inside a round — and it
     * reaches the same three-valued answer through the C ABI. */
    rc = purrdf_entail_entails(taxonomy, chain_axiom, 0, 1, &answer, &certificate,
                               &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_entails under a narrowed work cap");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen > strlen("entails unknown\n") &&
              memcmp(abytes, "entails unknown\n", strlen("entails unknown\n")) == 0,
          "a work-exhausted search is unknown, not false");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "completeness budget-exhausted\n") &&
              contains_bytes(cbytes, clen, "work-budget 1\n"),
          "the certificate reports the narrowed WORK budget it ran under");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_classify(taxonomy, 0, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_classify");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(contains_bytes(abytes, alen, "subclass <http://example.org/A> <http://example.org/C>\n"),
          "classification derives the transitive subsumption");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_realize(taxonomy, 0, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_realize");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_instances(taxonomy, "<http://example.org/C>", 0, 0, &answer,
                                 &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_instances");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen == strlen("instance <http://example.org/x>\n") &&
              memcmp(abytes, "instance <http://example.org/x>\n", alen) == 0,
          "instance retrieval reaches through the hierarchy");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_profile(taxonomy, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_profile");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(memcmp(abytes, "certified EL\n", strlen("certified EL\n")) == 0,
          "a bare sub-class taxonomy is in every profile, most restrictive first");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "one-directional true\n"),
          "a certification proves membership; a violation does not disprove it");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_extract_module(taxonomy, "<http://example.org/A>\n", "bot",
                                      &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_extract_module");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "method BOT\n"),
          "the extraction names the locality notion it used");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_justify(taxonomy, chain_axiom, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_justify");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "sufficient true\n") &&
              contains_bytes(cbytes, clen, "minimal true\n"),
          "the justification re-decides both halves of its claim");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_explain_conclusion(
        taxonomy, "owl-rl",
        "<http://example.org/x> "
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n",
        &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_explain_conclusion");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "checked true\n"),
          "the proof term re-derives against the clause program");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    /* the refusal path: neither out-param is written, so nothing is leaked */
    PurrdfError *dl_error = NULL;
    answer = NULL;
    certificate = NULL;
    rc = purrdf_entail_instances(taxonomy, "not a term", 0, 0, &answer, &certificate,
                                 &dl_error);
    CHECK(rc == PURRDF_STATUS_PARSE_ERROR, "a malformed class term is refused");
    CHECK(answer == NULL && certificate == NULL, "a failing DL call frees nothing");
    CHECK(dl_error != NULL && purrdf_error_message(dl_error) != NULL,
          "the refusal carries a message");
    purrdf_error_free(dl_error);

    /* ── the reasoning session ────────────────────────────────────────────
       The one-shot calls above each parse and reverse-map the ontology again.
       A session holds it. What is checked here is that the C surface REACHES
       the session at all — the six services were briefly generated by a Rust
       macro, which compiled and exported from the cdylib while cbindgen left
       them out of this header, so the code below would not have linked. */
    PurrdfReasoner *session = NULL;
    rc = purrdf_reasoner_open(taxonomy, 0, 0, &session, &error);
    CHECK(rc == PURRDF_STATUS_OK && session != NULL, "reasoner_open");

    /* Every service, through the handle, reusing ONE session: `instances` and
       `entails` mutate the shared knowledge base, so a later call answering
       differently would be state leaking between questions. */
    PurrdfBuffer *s_answer = NULL;
    PurrdfBuffer *s_cert = NULL;

    rc = purrdf_reasoner_consistency(session, &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_consistency");
    purrdf_buffer_data(s_answer, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "consistency true"), "the session decides");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_classify(session, &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_classify");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_realize(session, &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_realize");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_instances(session, "<http://example.org/C>", &s_answer,
                                   &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_instances");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_entails(session,
        "<http://example.org/A> "
        "<http://www.w3.org/2000/01/rdf-schema#subClassOf> "
        "<http://example.org/C> .\n",
        &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_entails");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_profile(session, &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_profile");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    rc = purrdf_reasoner_extract_module(session, "<http://example.org/C>", "star",
                                        &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_extract_module");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    /* The SAME question the first call answered, now after six siblings ran on
       this handle: the session must still say `consistency true`. */
    rc = purrdf_reasoner_consistency(session, &s_answer, &s_cert, &error);
    CHECK(rc == PURRDF_STATUS_OK, "reasoner_consistency after siblings");
    purrdf_buffer_data(s_answer, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "consistency true"),
          "a reused session does not change its answer");
    purrdf_buffer_free(s_answer);
    purrdf_buffer_free(s_cert);

    /* The refusal path through the handle writes neither out-param. */
    PurrdfError *sess_error = NULL;
    s_answer = NULL;
    s_cert = NULL;
    rc = purrdf_reasoner_instances(session, "not a term", &s_answer, &s_cert,
                                   &sess_error);
    CHECK(rc == PURRDF_STATUS_PARSE_ERROR, "the session refuses a malformed term");
    CHECK(s_answer == NULL && s_cert == NULL, "a failing session call frees nothing");
    purrdf_error_free(sess_error);

    purrdf_reasoner_free(session);
    purrdf_reasoner_free(NULL); /* documented no-op */

    purrdf_dataset_free(dataset);
    printf("C smoke OK\n");
    return 0;
}
