#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct DwgData DwgData;

enum { DWG_ENOTFOUND = 6 };

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_last_error_code(void);
extern int dwg_write_bytes(const DwgData *data, uint8_t **output, size_t *output_len);
extern int dwg_read_bytes(const uint8_t *input, size_t input_len, DwgData *data);
extern void dwg_free_buffer(void *buffer);

extern int dwg_data_named_objects_dictionary(const DwgData *data, uint64_t *handle);
extern int dwg_dictionary_count(const DwgData *data, uint64_t dictionary, size_t *count);
extern int dwg_dictionary_get(const DwgData *data, uint64_t dictionary, const char *key, uint64_t *value);
extern int dwg_dictionary_entry(const DwgData *data, uint64_t dictionary, size_t index,
                                char *key, size_t cap, size_t *needed, uint64_t *value);
extern int dwg_dictionary_set(DwgData *data, uint64_t dictionary, const char *key, uint64_t value);
extern int dwg_dictionary_remove(DwgData *data, uint64_t dictionary, const char *key, uint64_t *value);

int main(void) {
    DwgData *source = NULL;
    DwgData *roundtripped = NULL;
    uint8_t *bytes = NULL;
    size_t byte_count = 0;
    uint64_t source_root = 0;
    uint64_t group_handle = 0;
    uint64_t roundtripped_root = 0;
    uint64_t roundtripped_group = 0;
    uint64_t value = 0;
    size_t count = 0;
    size_t index;
    int found_test_entry = 0;
    int result = 0;

    if (dwg_data_create(&source) != 0) { result = 1; goto cleanup; }
    if (source == NULL) { result = 2; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 3; goto cleanup; }
    if (dwg_data_named_objects_dictionary(source, &source_root) != 0) { result = 4; goto cleanup; }
    if (source_root == 0) { result = 5; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 6; goto cleanup; }
    if (dwg_dictionary_get(source, source_root, "ACAD_GROUP", &group_handle) != 0) { result = 7; goto cleanup; }
    if (group_handle == 0) { result = 8; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 9; goto cleanup; }
    if (dwg_dictionary_count(source, source_root, &count) != 0) { result = 10; goto cleanup; }
    if (count == 0) { result = 11; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 12; goto cleanup; }
    if (dwg_dictionary_set(source, source_root, "P4B1_TEST", group_handle) != 0) { result = 13; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 14; goto cleanup; }

    if (dwg_dictionary_count(source, source_root, &count) != 0) { result = 15; goto cleanup; }
    for (index = 0; index < count; ++index) {
        char key[64];
        size_t needed = 0;

        if (dwg_dictionary_entry(source, source_root, index, NULL, 0, &needed, &value) != 0) {
            result = 16;
            goto cleanup;
        }
        if (needed == 0 || needed > sizeof key) { result = 17; goto cleanup; }
        if (dwg_dictionary_entry(source, source_root, index, key, sizeof key, &needed, &value) != 0) {
            result = 18;
            goto cleanup;
        }
        if (strcmp(key, "P4B1_TEST") == 0) {
            if (value != group_handle) { result = 19; goto cleanup; }
            found_test_entry = 1;
        }
    }
    if (!found_test_entry) { result = 20; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 21; goto cleanup; }

    if (dwg_write_bytes(source, &bytes, &byte_count) != 0) { result = 22; goto cleanup; }
    if (bytes == NULL || byte_count == 0) { result = 23; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 24; goto cleanup; }
    if (dwg_data_create(&roundtripped) != 0) { result = 25; goto cleanup; }
    if (roundtripped == NULL) { result = 26; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 27; goto cleanup; }
    if (dwg_read_bytes(bytes, byte_count, roundtripped) != 0) { result = 28; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 29; goto cleanup; }
    if (dwg_data_named_objects_dictionary(roundtripped, &roundtripped_root) != 0) { result = 30; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 31; goto cleanup; }
    if (dwg_dictionary_get(roundtripped, roundtripped_root, "ACAD_GROUP", &roundtripped_group) != 0) {
        result = 32;
        goto cleanup;
    }
    if (roundtripped_group != group_handle) { result = 33; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 34; goto cleanup; }
    if (dwg_dictionary_get(roundtripped, roundtripped_root, "P4B1_TEST", &value) != 0) {
        result = 35;
        goto cleanup;
    }
    if (value != group_handle) { result = 36; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 37; goto cleanup; }
    if (dwg_dictionary_remove(source, source_root, "P4B1_TEST", &value) != 0) { result = 38; goto cleanup; }
    if (value != group_handle) { result = 39; goto cleanup; }
    if (dwg_last_error_code() != 0) { result = 40; goto cleanup; }
    if (dwg_dictionary_get(source, source_root, "P4B1_TEST", &value) != DWG_ENOTFOUND) {
        result = 41;
        goto cleanup;
    }
    if (dwg_last_error_code() != DWG_ENOTFOUND) { result = 42; goto cleanup; }

cleanup:
    dwg_free_buffer(bytes);
    dwg_data_destroy(roundtripped);
    dwg_data_destroy(source);
    return result;
}
