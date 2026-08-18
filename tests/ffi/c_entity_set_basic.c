#include <stddef.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
  DWG_ETYPE = 8,
  DWG_OBJECT_LINE = 1,
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);
extern int dwg_entity_set_layer(DwgData *data, const DwgObject *obj,
                                const char *value);
extern int dwg_entity_line_set_start(DwgData *data, const DwgObject *obj,
                                     double x, double y, double z);
extern int dwg_entity_line_get_start(const DwgObject *obj, double *x, double *y,
                                     double *z);
extern int dwg_entity_circle_set_radius(DwgData *data, const DwgObject *obj,
                                        double radius);

int main(int argc, char **argv) {
  DwgData *data = NULL;
  const DwgObject *line = NULL;
  const DwgObject *obj = NULL;
  int error = 0;
  int exit_code = 0;
  double x = 0.0, y = 0.0, z = 0.0;

  if (argc < 2)
    return 1;
  if (dwg_data_create(&data) != 0)
    return 2;
  if (dwg_read_file(argv[1], data) != 0) {
    exit_code = 3;
    goto cleanup;
  }

  obj = dwg_data_first_object(data, &error);
  if (error != 0) {
    exit_code = 4;
    goto cleanup;
  }
  while (obj != NULL) {
    if (dwg_object_get_type(obj) == DWG_OBJECT_LINE) {
      line = obj;
      break;
    }
    obj = dwg_object_next(obj);
  }
  if (line == NULL) {
    exit_code = 5;
    goto cleanup;
  }

  if (dwg_entity_set_layer(data, line, "P3C_C") != 0) {
    exit_code = 10;
    goto cleanup;
  }
  if (dwg_entity_line_set_start(data, line, 31.0, 32.0, 33.0) != 0) {
    exit_code = 11;
    goto cleanup;
  }
  if (dwg_entity_line_get_start(line, &x, &y, &z) != 0) {
    exit_code = 12;
    goto cleanup;
  }
  if (x != 31.0 || y != 32.0 || z != 33.0) {
    exit_code = 13;
    goto cleanup;
  }
  if (dwg_entity_circle_set_radius(data, line, 2.0) != DWG_ETYPE) {
    exit_code = 14;
    goto cleanup;
  }

cleanup:
  dwg_data_destroy(data);
  return exit_code;
}
