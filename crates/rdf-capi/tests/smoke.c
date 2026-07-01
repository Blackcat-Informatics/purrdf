/* SPDX-License-Identifier: MIT OR Apache-2.0 */
/* A C smoke test for libpurrdf: it links the real C-ABI (header + shared
 * library), exercises a full round-trip, and returns non-zero on any failure.
 * Driven from tests/c_smoke.rs via the system C compiler. */

#include "purrdf.h"

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define CHECK(cond, msg)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            fprintf(stderr, "C smoke FAILED: %s (line %d)\n", (msg), __LINE__); \
            return 1;                                                           \
        }                                                                       \
    } while (0)

int main(void) {
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
