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

int main(int argc, char **argv) {
    CHECK(argc == 3, "shared OKF fixture and config arguments");
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

    purrdf_dataset_free(dataset);
    printf("C smoke OK\n");
    return 0;
}
