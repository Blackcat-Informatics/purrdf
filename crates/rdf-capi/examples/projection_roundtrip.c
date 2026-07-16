/* SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> */
/* SPDX-License-Identifier: MIT OR Apache-2.0 */

#include "purrdf.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static void print_error(const char *operation, PurrdfError *error) {
    const char *message = error == NULL ? "no diagnostic" : purrdf_error_message(error);
    fprintf(stderr, "%s failed: %s\n", operation, message == NULL ? "no diagnostic" : message);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s OUTPUT_USTAR\n", argv[0]);
        return 2;
    }
    const char *document =
        "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .\n";
    const char *config =
        "{\"profile\":\"lpg-csv\",\"config\":{\"rdf_type\":"
        "\"https://example.org/type\",\"limits\":{\"max_artifacts\":16,"
        "\"max_artifact_bytes\":1000000,\"max_total_bytes\":4000000,"
        "\"max_archive_bytes\":5000000,\"max_term_depth\":16},"
        "\"max_records\":1000}}";
    PurrdfDataset *dataset = NULL;
    PurrdfDataset *lifted = NULL;
    PurrdfBuffer *archive = NULL;
    PurrdfBuffer *project_ledger = NULL;
    PurrdfBuffer *lift_ledger = NULL;
    PurrdfError *error = NULL;
    int exit_code = 1;

    int status = purrdf_parse((const uint8_t *)document, strlen(document),
                              "text/turtle", NULL, NULL, &dataset, &error);
    if (status != PURRDF_STATUS_OK) {
        print_error("parse", error);
        goto cleanup;
    }
    status = purrdf_project(dataset, "lpg-csv", (const uint8_t *)config,
                            strlen(config), &archive, &project_ledger, &error);
    if (status != PURRDF_STATUS_OK) {
        print_error("project", error);
        goto cleanup;
    }

    const uint8_t *archive_bytes = NULL;
    size_t archive_len = 0;
    if (purrdf_buffer_data(archive, &archive_bytes, &archive_len) != PURRDF_STATUS_OK) {
        fputs("archive buffer access failed\n", stderr);
        goto cleanup;
    }
    FILE *file = fopen(argv[1], "wb");
    if (file == NULL) {
        fputs("archive write failed\n", stderr);
        goto cleanup;
    }
    size_t written = fwrite(archive_bytes, 1, archive_len, file);
    int close_status = fclose(file);
    if (written != archive_len || close_status != 0) {
        fputs("archive write failed\n", stderr);
        goto cleanup;
    }

    status = purrdf_lift(archive_bytes, archive_len, "lpg-csv",
                         (const uint8_t *)config, strlen(config), &lifted,
                         &lift_ledger, &error);
    if (status != PURRDF_STATUS_OK) {
        print_error("lift", error);
        goto cleanup;
    }
    size_t quad_count = 0;
    if (purrdf_dataset_quad_count(lifted, &quad_count) != PURRDF_STATUS_OK ||
        quad_count != 1) {
        fputs("projection round trip changed the RDF dataset\n", stderr);
        goto cleanup;
    }
    printf("wrote %zu bytes and lifted %zu quad\n", archive_len, quad_count);
    exit_code = 0;

cleanup:
    purrdf_error_free(error);
    purrdf_buffer_free(lift_ledger);
    purrdf_buffer_free(project_ledger);
    purrdf_buffer_free(archive);
    purrdf_dataset_free(lifted);
    purrdf_dataset_free(dataset);
    return exit_code;
}
