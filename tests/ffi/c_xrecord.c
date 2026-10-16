#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct DwgData DwgData;

typedef struct DwgXrecordValue {
    uint32_t tag;
    uint32_t reserved;
    int64_t integer;
    double real;
    double point[3];
    uint64_t handle;
    void *data;
    size_t data_len;
} DwgXrecordValue;

_Static_assert(offsetof(DwgXrecordValue, tag) == 0, "tag offset changed");
_Static_assert(offsetof(DwgXrecordValue, reserved) == 4, "reserved offset changed");
_Static_assert(offsetof(DwgXrecordValue, integer) == 8, "integer offset changed");
_Static_assert(offsetof(DwgXrecordValue, real) == 16, "real offset changed");
_Static_assert(offsetof(DwgXrecordValue, point) == 24, "point offset changed");
_Static_assert(offsetof(DwgXrecordValue, handle) == 48, "handle offset changed");
_Static_assert(offsetof(DwgXrecordValue, data) == 56, "data offset changed");
_Static_assert(offsetof(DwgXrecordValue, data_len) == 64, "data_len offset changed");
_Static_assert(sizeof(DwgXrecordValue) == 72, "DwgXrecordValue size changed");

enum {
    DWG_OK = 0,
    DWG_ENOTFOUND = 6,
    DWG_XRECORD_STRING = 1,
    DWG_XRECORD_DOUBLE = 2,
    DWG_XRECORD_INT16 = 3,
    DWG_XRECORD_INT32 = 4,
    DWG_XRECORD_INT64 = 5,
    DWG_XRECORD_BYTE = 6,
    DWG_XRECORD_BOOL = 7,
    DWG_XRECORD_HANDLE = 8,
    DWG_XRECORD_POINT3D = 9,
    DWG_XRECORD_CHUNK = 10
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern int dwg_write_bytes(const DwgData *data, uint8_t **output, size_t *output_len);
extern int dwg_read_bytes(const uint8_t *input, size_t input_len, DwgData *data);
extern void dwg_free_buffer(void *buffer);

extern int dwg_data_named_objects_dictionary(const DwgData *data, uint64_t *handle);
extern int dwg_dictionary_get(const DwgData *data, uint64_t dictionary, const char *key, uint64_t *value);

extern int dwg_xrecord_count(const DwgData *data, uint64_t xrecord_handle, size_t *count);
extern int dwg_xrecord_find(const DwgData *data, uint64_t xrecord_handle, int32_t code,
                            size_t occurrence, size_t *index);
extern int dwg_xrecord_entry(const DwgData *data, uint64_t xrecord_handle, size_t index,
                             int32_t *code, DwgXrecordValue *value);
extern int dwg_xrecord_append(DwgData *data, uint64_t xrecord_handle, int32_t code,
                              const DwgXrecordValue *value);
extern int dwg_xrecord_set(DwgData *data, uint64_t xrecord_handle, size_t index, int32_t code,
                           const DwgXrecordValue *value);
extern int dwg_xrecord_remove(DwgData *data, uint64_t xrecord_handle, size_t index,
                              int32_t *code, DwgXrecordValue *value);

static DwgXrecordValue empty_value(void) {
    DwgXrecordValue value;
    memset(&value, 0, sizeof value);
    return value;
}

static int read_entry(const DwgData *data, uint64_t xrecord, size_t index, int32_t *code,
                      DwgXrecordValue *value) {
    *value = empty_value();
    return dwg_xrecord_entry(data, xrecord, index, code, value);
}

static int verify_seeded_entry(const DwgData *data, uint64_t xrecord, size_t index) {
    int32_t code = 0;
    DwgXrecordValue value;
    if (read_entry(data, xrecord, index, &code, &value) != DWG_OK) return 100 + (int)index;

    switch (index) {
    case 0:
        return (code == 1 && value.tag == DWG_XRECORD_STRING &&
                value.data_len == sizeof("P4B2 string")) ? 0 : 110;
    case 1:
        return (code == 40 && value.tag == DWG_XRECORD_DOUBLE && value.real == 12.5) ? 0 : 111;
    case 2:
        return (code == 90 && value.tag == DWG_XRECORD_INT32 && value.integer == 123456) ? 0 : 112;
    case 3:
        return (code == 10 && value.tag == DWG_XRECORD_POINT3D &&
                value.point[0] == 1.0 && value.point[1] == 2.0 && value.point[2] == 3.0) ? 0 : 113;
    case 4:
        return (code == 310 && value.tag == DWG_XRECORD_CHUNK && value.data_len == 3) ? 0 : 114;
    default:
        return 115;
    }
}

static int verify_copied_string(const DwgData *data, uint64_t xrecord) {
    int32_t code = 0;
    char text[32];
    DwgXrecordValue value = empty_value();
    if (dwg_xrecord_entry(data, xrecord, 0, &code, &value) != DWG_OK) return 30;
    if (code != 1 || value.tag != DWG_XRECORD_STRING) return 31;
    if (value.data_len != sizeof("P4B2 string")) return 32;
    if (value.data_len > sizeof text) return 33;
    value.data = text;
    value.data_len = sizeof text;
    if (dwg_xrecord_entry(data, xrecord, 0, &code, &value) != DWG_OK) return 34;
    if (strcmp(text, "P4B2 string") != 0) return 35;
    if (text[value.data_len - 1] != '\0') return 36;
    return 0;
}

static int verify_copied_chunk(const DwgData *data, uint64_t xrecord) {
    int32_t code = 0;
    uint8_t bytes[3] = {0, 0, 0};
    DwgXrecordValue value = empty_value();
    if (dwg_xrecord_entry(data, xrecord, 4, &code, &value) != DWG_OK) return 40;
    if (code != 310 || value.tag != DWG_XRECORD_CHUNK || value.data_len != sizeof bytes) return 41;
    value.data = bytes;
    value.data_len = sizeof bytes;
    if (dwg_xrecord_entry(data, xrecord, 4, &code, &value) != DWG_OK) return 42;
    if (bytes[0] != 0xCA || bytes[1] != 0xFE || bytes[2] != 0x01) return 43;
    return 0;
}

static int load_xrecord(DwgData *data, const char *name, uint64_t *xrecord) {
    uint64_t root = 0;
    if (dwg_data_named_objects_dictionary(data, &root) != DWG_OK) return 20;
    if (root == 0) return 21;
    if (dwg_dictionary_get(data, root, name, xrecord) != DWG_OK) return 22;
    if (*xrecord == 0) return 23;
    return 0;
}

int main(int argc, char **argv) {
    DwgData *source = NULL;
    DwgData *foreign = NULL;
    DwgData *roundtripped = NULL;
    uint8_t *bytes = NULL;
    size_t byte_count = 0;
    uint64_t xrecord = 0;
    uint64_t foreign_xrecord = 0;
    uint64_t reopened_xrecord = 0;
    size_t count = 0;
    size_t index = 0;
    int32_t code = 0;
    int result = 0;
    size_t i;
    DwgXrecordValue value;

    if (argc < 3) { result = 1; goto cleanup; }
    if (dwg_data_create(&source) != DWG_OK || source == NULL) { result = 2; goto cleanup; }
    if (dwg_read_file(argv[1], source) != DWG_OK) { result = 3; goto cleanup; }
    result = load_xrecord(source, "P4B2_TEST", &xrecord);
    if (result != 0) goto cleanup;

    if (dwg_data_create(&foreign) != DWG_OK || foreign == NULL) { result = 24; goto cleanup; }
    if (dwg_read_file(argv[2], foreign) != DWG_OK) { result = 25; goto cleanup; }
    result = load_xrecord(foreign, "P4B2_FOREIGN", &foreign_xrecord);
    if (result != 0) goto cleanup;

    if (dwg_xrecord_count(source, xrecord, &count) != DWG_OK) { result = 4; goto cleanup; }
    if (count != 5) { result = 5; goto cleanup; }
    for (i = 0; i < count; ++i) {
        result = verify_seeded_entry(source, xrecord, i);
        if (result != 0) goto cleanup;
    }

    value = empty_value();
    value.tag = DWG_XRECORD_HANDLE;
    value.handle = foreign_xrecord;
    if (dwg_xrecord_append(source, xrecord, 330, &value) != DWG_ENOTFOUND) {
        result = 26;
        goto cleanup;
    }
    if (dwg_xrecord_count(source, xrecord, &count) != DWG_OK || count != 5) {
        result = 27;
        goto cleanup;
    }
    for (i = 0; i < count; ++i) {
        result = verify_seeded_entry(source, xrecord, i);
        if (result != 0) goto cleanup;
    }
    if (dwg_xrecord_find(source, xrecord, 40, 0, &index) != DWG_OK) { result = 6; goto cleanup; }
    if (index != 1) { result = 7; goto cleanup; }
    if (dwg_xrecord_find(source, xrecord, 90, 0, &index) != DWG_OK) { result = 8; goto cleanup; }
    if (index != 2) { result = 9; goto cleanup; }

    value = empty_value();
    value.tag = DWG_XRECORD_INT32;
    value.integer = 654321;
    if (dwg_xrecord_append(source, xrecord, 90, &value) != DWG_OK) { result = 10; goto cleanup; }

    value = empty_value();
    value.tag = DWG_XRECORD_DOUBLE;
    value.real = 42.25;
    if (dwg_xrecord_set(source, xrecord, 1, 40, &value) != DWG_OK) { result = 11; goto cleanup; }

    result = verify_copied_string(source, xrecord);
    if (result != 0) goto cleanup;
    result = verify_copied_chunk(source, xrecord);
    if (result != 0) goto cleanup;

    if (dwg_xrecord_count(source, xrecord, &count) != DWG_OK) { result = 12; goto cleanup; }
    if (count != 6) { result = 13; goto cleanup; }
    if (dwg_xrecord_find(source, xrecord, 90, 1, &index) != DWG_OK) { result = 14; goto cleanup; }
    if (index != 5) { result = 15; goto cleanup; }

    if (dwg_write_bytes(source, &bytes, &byte_count) != DWG_OK) { result = 16; goto cleanup; }
    if (bytes == NULL || byte_count == 0) { result = 17; goto cleanup; }
    if (dwg_data_create(&roundtripped) != DWG_OK || roundtripped == NULL) { result = 18; goto cleanup; }
    if (dwg_read_bytes(bytes, byte_count, roundtripped) != DWG_OK) { result = 19; goto cleanup; }
    result = load_xrecord(roundtripped, "P4B2_TEST", &reopened_xrecord);
    if (result != 0) goto cleanup;

    if (dwg_xrecord_count(roundtripped, reopened_xrecord, &count) != DWG_OK) { result = 50; goto cleanup; }
    if (count != 6) { result = 51; goto cleanup; }
    result = verify_copied_string(roundtripped, reopened_xrecord);
    if (result != 0) goto cleanup;
    result = verify_copied_chunk(roundtripped, reopened_xrecord);
    if (result != 0) goto cleanup;
    if (read_entry(roundtripped, reopened_xrecord, 1, &code, &value) != DWG_OK) { result = 52; goto cleanup; }
    if (code != 40 || value.tag != DWG_XRECORD_DOUBLE || value.real != 42.25) { result = 53; goto cleanup; }
    if (read_entry(roundtripped, reopened_xrecord, 5, &code, &value) != DWG_OK) { result = 54; goto cleanup; }
    if (code != 90 || value.tag != DWG_XRECORD_INT32 || value.integer != 654321) { result = 55; goto cleanup; }

    value = empty_value();
    if (dwg_xrecord_remove(roundtripped, reopened_xrecord, 5, &code, &value) != DWG_OK) {
        result = 56;
        goto cleanup;
    }
    if (code != 90 || value.tag != DWG_XRECORD_INT32 || value.integer != 654321) {
        result = 57;
        goto cleanup;
    }
    if (dwg_xrecord_count(roundtripped, reopened_xrecord, &count) != DWG_OK) { result = 58; goto cleanup; }
    if (count != 5) { result = 59; goto cleanup; }

cleanup:
    dwg_free_buffer(bytes);
    dwg_data_destroy(roundtripped);
    dwg_data_destroy(foreign);
    dwg_data_destroy(source);
    return result;
}
