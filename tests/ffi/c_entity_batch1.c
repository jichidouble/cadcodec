#include <stdint.h>
#include <stdio.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern int dwg_entity_get_layer(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
extern int dwg_entity_ellipse_get_center(const DwgObject *obj, double *x, double *y, double *z);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;

    int err = 0;
    const DwgObject *obj = dwg_data_first_object(data, &err);
    if (err != 0 || obj == NULL) return 2;

    /* A fresh document's first object is a non-entity (default dictionary):
       entity getters must report DWG_ETYPE, never crash. */
    char buf[64];
    size_t needed = 0;
    if (dwg_entity_get_layer(obj, buf, sizeof buf, &needed) != 8) return 3; /* DWG_ETYPE */
    double x = 0, y = 0, z = 0;
    if (dwg_entity_ellipse_get_center(obj, &x, &y, &z) != 8) return 4; /* DWG_ETYPE */

    /* NULL handling. */
    if (dwg_entity_ellipse_get_center(NULL, &x, &y, &z) != 1) return 5; /* DWG_EINVAL */

    dwg_data_destroy(data);
    return 0;
}
