#include <stddef.h>
#include <stdint.h>

typedef struct DwgData DwgData;

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_data_version(const DwgData *data, char *buf, size_t cap, size_t *needed);
extern int dwg_read_bytes(const uint8_t *input, size_t input_len, DwgData *data);
extern int dwg_write_bytes(const DwgData *data, uint8_t **output, size_t *output_len);
extern void dwg_free_buffer(void *buffer);

int main(void) {
    DwgData *source = NULL;
    DwgData *roundtripped = NULL;
    uint8_t *bytes = NULL;
    size_t byte_count = 0;
    char version[16];
    size_t version_needed = 0;

    if (dwg_data_create(&source) != 0 || source == NULL) return 1;
    if (dwg_write_bytes(source, &bytes, &byte_count) != 0) return 2;
    if (bytes == NULL || byte_count == 0) return 3;
    if (dwg_data_create(&roundtripped) != 0 || roundtripped == NULL) return 4;
    if (dwg_read_bytes(bytes, byte_count, roundtripped) != 0) return 5;
    if (dwg_data_version(roundtripped, version, sizeof version, &version_needed) != 0) return 6;
    if (version_needed != 7 || version[0] != 'A' || version[1] != 'C' ||
        version[2] != '1' || version[3] != '0' || version[4] != '3' ||
        version[5] != '2' || version[6] != '\0') return 7;

    dwg_free_buffer(bytes);
    dwg_free_buffer(NULL);
    dwg_data_destroy(roundtripped);
    dwg_data_destroy(source);
    return 0;
}
