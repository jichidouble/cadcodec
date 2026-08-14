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

    dwg_data_destroy(data);
    return 0;
}
