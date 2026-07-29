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

int main(int argc, char **argv) {
    CHECK(argc == 4,
          "shared OKF fixture, OKF config, and entailment golden vector arguments");
    /* ABI version */
    PurrdfAbiVersion version;
    CHECK(purrdf_abi_version(&version) == PURRDF_STATUS_OK, "abi_version");
    printf("libpurrdf ABI %u.%u.%u\n", version.major, version.minor, version.patch);
    CHECK(version.major == 0 && version.minor == 1, "abi 0.1.x");

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
    rc = purrdf_serialize(dataset, "application/n-triples", NULL, &serialized,
                          &dropped, &error);
    CHECK(rc == PURRDF_STATUS_OK && serialized != NULL, "serialize");
    CHECK(dropped == 0, "no statement rows dropped for n-triples");
    const uint8_t *sbytes = NULL;
    size_t slen = 0;
    CHECK(purrdf_buffer_data(serialized, &sbytes, &slen) == PURRDF_STATUS_OK,
          "buffer_data");
    CHECK(slen > 0, "serialized bytes present");
    purrdf_buffer_free(serialized);

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
    rc = purrdf_query_json(dataset, "SELECT ?s WHERE { ?s ?p ?o }", NULL, &json,
                           &error);
    CHECK(rc == PURRDF_STATUS_OK && json != NULL, "query_json");
    const uint8_t *jbytes = NULL;
    size_t jlen = 0;
    purrdf_buffer_data(json, &jbytes, &jlen);
    CHECK(jlen > 0, "sparql-json bytes present");
    purrdf_buffer_free(json);

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

    rc = purrdf_entail_consistency(taxonomy, 0, &answer, &certificate, &error);
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

    rc = purrdf_entail_entails(taxonomy, chain_axiom, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_entails");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(alen > strlen("entails true\n") &&
              memcmp(abytes, "entails true\n", strlen("entails true\n")) == 0,
          "the chain entails A subClassOf C");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    /* a narrowed step cap answers `unknown`, never `false`: the third
     * completeness state, reachable through the C ABI. */
    rc = purrdf_entail_entails(taxonomy, chain_axiom, 1, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_entails under a narrowed cap");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(memcmp(abytes, "entails unknown\n", strlen("entails unknown\n")) == 0,
          "an exhausted search is unknown, not false");
    purrdf_buffer_data(certificate, &cbytes, &clen);
    CHECK(contains_bytes(cbytes, clen, "completeness budget-exhausted\n"),
          "the certificate says the budget ran out");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_classify(taxonomy, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_classify");
    purrdf_buffer_data(answer, &abytes, &alen);
    CHECK(contains_bytes(abytes, alen, "subclass <http://example.org/A> <http://example.org/C>\n"),
          "classification derives the transitive subsumption");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_realize(taxonomy, 0, &answer, &certificate, &error);
    CHECK(rc == PURRDF_STATUS_OK, "entail_realize");
    purrdf_buffer_free(certificate);
    purrdf_buffer_free(answer);

    rc = purrdf_entail_instances(taxonomy, "<http://example.org/C>", 0, &answer,
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
    rc = purrdf_entail_instances(taxonomy, "not a term", 0, &answer, &certificate,
                                 &dl_error);
    CHECK(rc == PURRDF_STATUS_PARSE_ERROR, "a malformed class term is refused");
    CHECK(answer == NULL && certificate == NULL, "a failing DL call frees nothing");
    CHECK(dl_error != NULL && purrdf_error_message(dl_error) != NULL,
          "the refusal carries a message");
    purrdf_error_free(dl_error);

    purrdf_dataset_free(dataset);
    printf("C smoke OK\n");
    return 0;
}
