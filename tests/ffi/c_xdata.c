#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct DwgData DwgData;

typedef struct DwgXdataValue {
    uint32_t tag;
    uint32_t reserved;
    int64_t integer;
    double real;
    double point[3];
    uint64_t handle;
    void *data;
    size_t data_len;
} DwgXdataValue;

_Static_assert(offsetof(DwgXdataValue, tag) == 0, "tag offset changed");
_Static_assert(offsetof(DwgXdataValue, reserved) == 4, "reserved offset changed");
_Static_assert(offsetof(DwgXdataValue, integer) == 8, "integer offset changed");
_Static_assert(offsetof(DwgXdataValue, real) == 16, "real offset changed");
_Static_assert(offsetof(DwgXdataValue, point) == 24, "point offset changed");
_Static_assert(offsetof(DwgXdataValue, handle) == 48, "handle offset changed");
_Static_assert(offsetof(DwgXdataValue, data) == 56, "data offset changed");
_Static_assert(offsetof(DwgXdataValue, data_len) == 64, "data_len offset changed");
_Static_assert(sizeof(DwgXdataValue) == 72, "DwgXdataValue size changed");

enum {
    DWG_OK = 0, DWG_EINVAL = 1, DWG_ENOTFOUND = 6, DWG_EBUFFER = 7,
    DWG_ETYPE = 8,
    DWG_XDATA_STRING = 1, DWG_XDATA_CONTROL_STRING = 2,
    DWG_XDATA_LAYER_NAME = 3, DWG_XDATA_BINARY = 4, DWG_XDATA_HANDLE = 5,
    DWG_XDATA_POINT3D = 6, DWG_XDATA_POSITION3D = 7,
    DWG_XDATA_DISPLACEMENT3D = 8, DWG_XDATA_DIRECTION3D = 9,
    DWG_XDATA_REAL = 10, DWG_XDATA_DISTANCE = 11,
    DWG_XDATA_SCALE_FACTOR = 12, DWG_XDATA_INTEGER16 = 13,
    DWG_XDATA_INTEGER32 = 14
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern int dwg_last_error_code(void);

extern int dwg_entity_xdata_record_count(const DwgData *, uint64_t, size_t *);
extern int dwg_entity_xdata_record_find(const DwgData *, uint64_t, const char *, size_t, size_t *);
extern int dwg_entity_xdata_record_entry(const DwgData *, uint64_t, size_t, char *, size_t, size_t *, size_t *);
extern int dwg_entity_xdata_record_append(DwgData *, uint64_t, const char *, size_t *);
extern int dwg_entity_xdata_record_remove(DwgData *, uint64_t, size_t, char *, size_t, size_t *);
extern int dwg_entity_xdata_value(const DwgData *, uint64_t, size_t, size_t, DwgXdataValue *);
extern int dwg_entity_xdata_value_append(DwgData *, uint64_t, size_t, const DwgXdataValue *);
extern int dwg_entity_xdata_value_set(DwgData *, uint64_t, size_t, size_t, const DwgXdataValue *);
extern int dwg_entity_xdata_value_remove(DwgData *, uint64_t, size_t, size_t, DwgXdataValue *);

static DwgXdataValue empty_value(void) {
    DwgXdataValue value;
    memset(&value, 0, sizeof value);
    return value;
}

static int read_value(const DwgData *data, uint64_t entity, size_t index,
                      DwgXdataValue *value, unsigned char *buffer, size_t capacity) {
    *value = empty_value();
    if (dwg_entity_xdata_value(data, entity, 0, index, value) != DWG_OK) return 20 + (int)index;
    if (value->data_len != 0) {
        if (value->data_len > capacity) return 40 + (int)index;
        value->data = buffer;
        value->data_len = capacity;
        if (dwg_entity_xdata_value(data, entity, 0, index, value) != DWG_OK) return 60 + (int)index;
    }
    return 0;
}

static int verify_seeded(const DwgData *data, uint64_t entity, uint64_t object) {
    size_t count = 0, index = 99, required = 0, values = 0;
    char name[32];
    if (dwg_entity_xdata_record_count(data, entity, &count) != DWG_OK || count != 1) return 70;
    if (dwg_entity_xdata_record_find(data, entity, "P4B3_APP", 0, &index) != DWG_OK || index != 0) return 71;
    if (dwg_entity_xdata_record_entry(data, entity, 0, name, sizeof name, &required, &values) != DWG_OK ||
        strcmp(name, "P4B3_APP") != 0 || required != sizeof("P4B3_APP") || values != 14) return 72;

    DwgXdataValue value;
    unsigned char bytes[64];
    int rc = read_value(data, entity, 0, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_STRING || strcmp((char *)bytes, "P4B3 string") != 0) return 80 + rc;
    rc = read_value(data, entity, 1, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_CONTROL_STRING || strcmp((char *)bytes, "{") != 0) return 90 + rc;
    rc = read_value(data, entity, 2, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_LAYER_NAME || strcmp((char *)bytes, "P4B3_LAYER") != 0) return 100 + rc;
    rc = read_value(data, entity, 3, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_BINARY || value.data_len != 3 || bytes[0] != 0xca || bytes[1] != 0xfe || bytes[2] != 1) return 110 + rc;
    rc = read_value(data, entity, 4, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_HANDLE || value.handle != object) return 120 + rc;
    rc = read_value(data, entity, 5, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_POINT3D || value.point[2] != 3.0) return 130 + rc;
    rc = read_value(data, entity, 6, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_POSITION3D || value.point[2] != 6.0) return 140 + rc;
    rc = read_value(data, entity, 7, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_DISPLACEMENT3D || value.point[2] != 9.0) return 150 + rc;
    rc = read_value(data, entity, 8, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_DIRECTION3D || value.point[2] != 12.0) return 160 + rc;
    rc = read_value(data, entity, 9, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_REAL || value.real != 13.25) return 170 + rc;
    rc = read_value(data, entity, 10, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_DISTANCE || value.real != 14.5) return 180 + rc;
    rc = read_value(data, entity, 11, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_SCALE_FACTOR || value.real != 15.75) return 190 + rc;
    rc = read_value(data, entity, 12, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_INTEGER16 || value.integer != -321) return 200 + rc;
    rc = read_value(data, entity, 13, &value, bytes, sizeof bytes);
    if (rc || value.tag != DWG_XDATA_INTEGER32 || value.integer != 123456) return 210 + rc;
    return 0;
}

int main(int argc, char **argv) {
    DwgData *data = NULL, *foreign = NULL;
    uint64_t entity = 0, object = 0;
    size_t count = 0, index = 0, required = 0, values = 0;
    int result = 0;
    if (argc < 5 || dwg_data_create(&data) != DWG_OK || dwg_data_create(&foreign) != DWG_OK) return 1;
    if (dwg_read_file(argv[1], data) != DWG_OK || dwg_read_file(argv[2], foreign) != DWG_OK) { result = 2; goto cleanup; }
    entity = strtoull(argv[3], NULL, 0);
    object = strtoull(argv[4], NULL, 0);
    result = verify_seeded(data, entity, object);
    if (result) goto cleanup;

    if (dwg_entity_xdata_record_count(data, UINT64_MAX, &count) != DWG_ENOTFOUND) { result = 4; goto cleanup; }
    DwgXdataValue value = empty_value();
    value.tag = DWG_XDATA_HANDLE;
    value.handle = object;
    if (dwg_entity_xdata_value_append(data, entity, 0, &value) != DWG_OK) { result = 5; goto cleanup; }
    if (dwg_entity_xdata_record_entry(data, entity, 0, NULL, 0, &required, &values) != DWG_OK || values != 15) { result = 6; goto cleanup; }
    if (dwg_entity_xdata_value_remove(data, entity, 0, 14, &value) != DWG_OK || value.tag != DWG_XDATA_HANDLE) { result = 7; goto cleanup; }

    value = empty_value();
    value.tag = DWG_XDATA_REAL;
    value.real = 99.5;
    if (dwg_entity_xdata_value_set(data, entity, 0, 9, &value) != DWG_OK) { result = 8; goto cleanup; }
    value = empty_value();
    if (dwg_entity_xdata_value(data, entity, 0, 9, &value) != DWG_OK || value.tag != DWG_XDATA_REAL || value.real != 99.5) { result = 9; goto cleanup; }

    size_t empty_index = 0;
    if (dwg_entity_xdata_record_append(data, entity, "C_SMOKE_EMPTY", &empty_index) != DWG_OK || empty_index != 1) { result = 10; goto cleanup; }
    char removed_name[32];
    if (dwg_entity_xdata_record_remove(data, entity, empty_index, removed_name, sizeof removed_name, &required) != DWG_OK || strcmp(removed_name, "C_SMOKE_EMPTY") != 0) { result = 11; goto cleanup; }
    value = empty_value();
    value.tag = DWG_XDATA_INTEGER32;
    value.integer = 1;
    if (dwg_entity_xdata_value_append(data, entity, 0, &value) != DWG_OK) { result = 12; goto cleanup; }
    if (dwg_entity_xdata_value_remove(data, entity, 0, 14, &value) != DWG_OK || value.integer != 1) { result = 13; goto cleanup; }
    result = dwg_last_error_code() == DWG_OK ? 0 : 14;

cleanup:
    dwg_data_destroy(foreign);
    dwg_data_destroy(data);
    return result;
}
