#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct DwgData DwgData;

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_last_error(char *buf, size_t cap, size_t *needed);
extern int dwg_last_error_code(void);
extern int dwg_data_version(const DwgData *data, char *buf, size_t cap, size_t *needed);
extern int dwg_read_file(const char *path, DwgData *data);
extern int dwg_write_file(const char *path, const DwgData *data);
extern int dxf_read_file(const char *path, DwgData *data);
extern int dxf_write_file(const char *path, const DwgData *data);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;
    if (data == NULL) return 2;

    char buf[64];
    size_t needed = 0;
    int rc = dwg_data_version(data, buf, sizeof buf, &needed);
    if (rc != 0) return 3;
    if (strcmp(buf, "AC1032") != 0) return 4;

    rc = dwg_last_error(buf, sizeof buf, &needed);
    if (rc != 0) return 5;

    /* Read/write round-trip through the C API: write -> read -> write -> read */
    const char *dwg_path = "c_smoke_rt.dwg";
    const char *dxf_path = "c_smoke_rt.dxf";
    rc = dwg_write_file(dwg_path, data);
    if (rc != 0) return 6;
    rc = dwg_read_file(dwg_path, data);
    if (rc != 0) return 7;
    rc = dxf_write_file(dxf_path, data);
    if (rc != 0) return 8;
    rc = dxf_read_file(dxf_path, data);
    if (rc != 0) return 9;
    remove(dwg_path);
    remove(dxf_path);

    dwg_data_destroy(data);
    return 0;
}
