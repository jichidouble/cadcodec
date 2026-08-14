/* Entity-value smoke test: opens a DWG fixture and verifies typed getters
 * return the expected geometry/text values through the C ABI.
 *
 * Usage: c_entities <fixture.dwg>
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
    DWG_OBJECT_LINE = 1,
    DWG_OBJECT_CIRCLE = 2,
    DWG_OBJECT_TEXT = 4,
    DWG_OBJECT_ELLIPSE = 6,
    DWG_OBJECT_MTEXT = 7,
    DWG_OBJECT_LWPOLYLINE = 9,
    DWG_OBJECT_HATCH = 20,
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);
extern int dwg_entity_line_get_start(const DwgObject *obj, double *x, double *y, double *z);
extern int dwg_entity_line_get_end(const DwgObject *obj, double *x, double *y, double *z);
extern int dwg_entity_circle_get_center(const DwgObject *obj, double *x, double *y, double *z);
extern int dwg_entity_circle_get_radius(const DwgObject *obj, double *r);
extern int dwg_entity_ellipse_get_minor_axis_ratio(const DwgObject *obj, double *r);
extern int dwg_entity_mtext_get_value(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
extern int dwg_entity_lwpolyline_get_vertex_count(const DwgObject *obj, int *count);
extern int dwg_entity_hatch_get_pattern_name(const DwgObject *obj, char *buf, size_t cap, size_t *needed);

#define NEAR(a, b) (((a) - (b)) < 1e-9 && ((b) - (a)) < 1e-9)

static const DwgObject *find_type(const DwgData *data, int want) {
    int err = 0;
    const DwgObject *obj = dwg_data_first_object(data, &err);
    if (err != 0) return NULL;
    while (obj != NULL) {
        if (dwg_object_get_type(obj) == want) return obj;
        obj = dwg_object_next(obj);
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;

    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 2;
    if (dwg_read_file(argv[1], data) != 0) return 3;

    double x = 0, y = 0, z = 0;
    char buf[128];
    size_t needed = 0;

    const DwgObject *line = find_type(data, DWG_OBJECT_LINE);
    if (line == NULL) return 4;
    if (dwg_entity_line_get_start(line, &x, &y, &z) != 0) return 5;
    if (!NEAR(x, 0.0) || !NEAR(y, 0.0) || !NEAR(z, 0.0)) return 6;
    if (dwg_entity_line_get_end(line, &x, &y, &z) != 0) return 7;
    if (!NEAR(x, 10.0) || !NEAR(y, 20.0) || !NEAR(z, 30.0)) return 8;

    const DwgObject *circle = find_type(data, DWG_OBJECT_CIRCLE);
    if (circle == NULL) return 9;
    if (dwg_entity_circle_get_center(circle, &x, &y, &z) != 0) return 10;
    if (!NEAR(x, 5.0) || !NEAR(y, 6.0)) return 11;
    double r = 0;
    if (dwg_entity_circle_get_radius(circle, &r) != 0) return 12;
    if (!NEAR(r, 2.5)) return 13;

    const DwgObject *ell = find_type(data, DWG_OBJECT_ELLIPSE);
    if (ell == NULL) return 14;
    if (dwg_entity_ellipse_get_minor_axis_ratio(ell, &r) != 0) return 15;
    if (!NEAR(r, 0.5)) return 16;

    const DwgObject *mtx = find_type(data, DWG_OBJECT_MTEXT);
    if (mtx == NULL) return 17;
    if (dwg_entity_mtext_get_value(mtx, buf, sizeof buf, &needed) != 0) return 18;
    if (strcmp(buf, "mt") != 0) return 19;

    const DwgObject *lw = find_type(data, DWG_OBJECT_LWPOLYLINE);
    if (lw == NULL) return 20;
    int count = 0;
    if (dwg_entity_lwpolyline_get_vertex_count(lw, &count) != 0) return 21;
    if (count != 2) return 22;

    const DwgObject *h = find_type(data, DWG_OBJECT_HATCH);
    if (h == NULL) return 23;
    if (dwg_entity_hatch_get_pattern_name(h, buf, sizeof buf, &needed) != 0) return 24;
    if (strcmp(buf, "SOLID") != 0) return 25;

    dwg_data_destroy(data);
    return 0;
}
