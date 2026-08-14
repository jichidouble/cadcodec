#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct DwgData DwgData;

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_layer_table_count(const DwgData *data, int *count);
extern int dwg_layer_get_name(const DwgData *data, int index, char *buf, size_t cap, size_t *needed);
extern int dwg_layer_get_color(const DwgData *data, int index, int32_t *color);
extern int dwg_layer_set_color(DwgData *data, int index, int32_t color);
extern int dwg_layer_index_by_name(const DwgData *data, const char *name, int *index);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;

    int count = 0;
    if (dwg_layer_table_count(data, &count) != 0) return 2;
    if (count < 1) return 3;

    char name[64];
    size_t needed = 0;
    if (dwg_layer_get_name(data, 0, name, sizeof name, &needed) != 0) return 4;
    if (strcmp(name, "0") != 0) return 5;

    int32_t color = 0x0000FF00; /* green */
    if (dwg_layer_set_color(data, 0, color) != 0) return 6;
    int32_t read = 0;
    if (dwg_layer_get_color(data, 0, &read) != 0) return 7;
    if (read != color) return 8;

    int index = -1;
    if (dwg_layer_index_by_name(data, "0", &index) != 0) return 9;
    if (index != 0) return 10;

    dwg_data_destroy(data);
    return 0;
}
