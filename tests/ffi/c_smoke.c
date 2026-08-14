/* Minimal C consumer smoke test for the acadrust FFI (Phase 1 surface).
 *
 * Exercises the lifecycle, version, and error-slot entry points exactly as
 * an external C program would after reading dwg_api.h.
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Opaque handle; definition lives in the library. */
typedef struct DwgData DwgData;

/* dwg_data_create / destroy */
extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);

/* Version query */
extern int dwg_data_version(const DwgData *data, char *buf, size_t cap, size_t *needed);

/* Error slot */
extern int dwg_last_error(char *buf, size_t cap, size_t *needed);
extern int dwg_last_error_code(void);

int main(void) {
    DwgData *data = NULL;
    char buf[256];
    size_t n = 0;

    if (dwg_data_create(&data) != 0) {
        fprintf(stderr, "dwg_data_create failed\n");
        return 1;
    }
    if (data == NULL) {
        fprintf(stderr, "dwg_data_create returned NULL handle\n");
        return 2;
    }

    /* Error slot is empty after a successful create. */
    if (dwg_last_error(buf, sizeof buf, &n) != 0 || n != 1 || buf[0] != '\0') {
        fprintf(stderr, "error slot not empty after create (n=%zu)\n", n);
        return 3;
    }
    if (dwg_last_error_code() != 0) {
        fprintf(stderr, "error code not DWG_OK after create\n");
        return 4;
    }

    /* Version of a fresh document is a NUL-terminated string. */
    if (dwg_data_version(data, buf, sizeof buf, &n) != 0 || n == 0 || buf[n - 1] != '\0') {
        fprintf(stderr, "version query failed (n=%zu)\n", n);
        return 5;
    }

    /* NULL data must be rejected (DWG_EINVAL == 1). */
    if (dwg_data_version(NULL, buf, sizeof buf, &n) != 1) {
        fprintf(stderr, "NULL version query was not DWG_EINVAL\n");
        return 6;
    }

    dwg_data_destroy(data);
    return 0;
}
