/* Fixed-field entity setter smoke test.
 *
 * Usage: c_entity_set_fixed <fixture.dwg>
 */
#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
  DWG_ETYPE = 8,
  DWG_OBJECT_TEXT = 4,
  DWG_OBJECT_INSERT = 14,
  DWG_OBJECT_DIMENSION = 27,
  DWG_OBJECT_RASTER_IMAGE = 28,
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data,
                                               int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);
extern int dwg_entity_text_set_value(DwgData *data, const DwgObject *obj,
                                     const char *value);
extern int dwg_entity_text_get_value(const DwgObject *obj, char *buf,
                                     size_t cap, size_t *needed);
extern int dwg_entity_insert_set_array(DwgData *data, const DwgObject *obj,
                                       uint16_t columns, uint16_t rows,
                                       double column_spacing,
                                       double row_spacing);
extern int dwg_entity_insert_get_array(const DwgObject *obj,
                                       uint16_t *columns, uint16_t *rows);
extern int dwg_entity_dimension_set_style_name(DwgData *data,
                                               const DwgObject *obj,
                                               const char *value);
extern int dwg_entity_dimension_get_style_name(const DwgObject *obj, char *buf,
                                               size_t cap, size_t *needed);
extern int dwg_entity_raster_image_set_size(DwgData *data,
                                            const DwgObject *obj, double width,
                                            double height);
extern int dwg_entity_raster_image_get_size(const DwgObject *obj,
                                            double *width, double *height);

int main(int argc, char **argv) {
  DwgData *data = NULL;
  const DwgObject *text = NULL;
  const DwgObject *insert = NULL;
  const DwgObject *dimension = NULL;
  const DwgObject *image = NULL;
  const DwgObject *obj = NULL;
  int error = 0;
  int exit_code = 0;
  char buf[64];
  size_t needed = 0;
  uint16_t columns = 0;
  uint16_t rows = 0;
  double width = 0.0;
  double height = 0.0;

  if (argc < 2)
    return 1;
  if (dwg_data_create(&data) != 0)
    return 2;
  if (data == NULL) {
    exit_code = 3;
    goto cleanup;
  }
  if (dwg_read_file(argv[1], data) != 0) {
    exit_code = 4;
    goto cleanup;
  }

  obj = dwg_data_first_object(data, &error);
  if (error != 0) {
    exit_code = 5;
    goto cleanup;
  }
  while (obj != NULL) {
    switch (dwg_object_get_type(obj)) {
    case DWG_OBJECT_TEXT:
      text = obj;
      break;
    case DWG_OBJECT_INSERT:
      insert = obj;
      break;
    case DWG_OBJECT_DIMENSION:
      dimension = obj;
      break;
    case DWG_OBJECT_RASTER_IMAGE:
      image = obj;
      break;
    default:
      break;
    }
    obj = dwg_object_next(obj);
  }
  if (text == NULL) {
    exit_code = 6;
    goto cleanup;
  }
  if (insert == NULL) {
    exit_code = 7;
    goto cleanup;
  }
  if (dimension == NULL) {
    exit_code = 8;
    goto cleanup;
  }
  if (image == NULL) {
    exit_code = 9;
    goto cleanup;
  }

  if (dwg_entity_text_set_value(data, text, "C P3C TEXT") != 0) {
    exit_code = 10;
    goto cleanup;
  }
  if (dwg_entity_text_get_value(text, buf, sizeof buf, &needed) != 0) {
    exit_code = 11;
    goto cleanup;
  }
  if (needed != 11) {
    exit_code = 12;
    goto cleanup;
  }
  if (strcmp(buf, "C P3C TEXT") != 0) {
    exit_code = 13;
    goto cleanup;
  }

  if (dwg_entity_insert_set_array(data, insert, 2, 3, 4.0, 5.0) != 0) {
    exit_code = 14;
    goto cleanup;
  }
  if (dwg_entity_insert_get_array(insert, &columns, &rows) != 0) {
    exit_code = 15;
    goto cleanup;
  }
  if (columns != 2) {
    exit_code = 16;
    goto cleanup;
  }
  if (rows != 3) {
    exit_code = 17;
    goto cleanup;
  }

  needed = 0;
  if (dwg_entity_dimension_set_style_name(data, dimension, "C_P3C_DIM") != 0) {
    exit_code = 18;
    goto cleanup;
  }
  if (dwg_entity_dimension_get_style_name(dimension, buf, sizeof buf,
                                          &needed) != 0) {
    exit_code = 19;
    goto cleanup;
  }
  if (needed != 10) {
    exit_code = 20;
    goto cleanup;
  }
  if (strcmp(buf, "C_P3C_DIM") != 0) {
    exit_code = 21;
    goto cleanup;
  }

  if (dwg_entity_raster_image_set_size(data, image, 800.0, 600.0) != 0) {
    exit_code = 22;
    goto cleanup;
  }
  if (dwg_entity_raster_image_get_size(image, &width, &height) != 0) {
    exit_code = 23;
    goto cleanup;
  }
  if (width != 800.0) {
    exit_code = 24;
    goto cleanup;
  }
  if (height != 600.0) {
    exit_code = 25;
    goto cleanup;
  }
  if (dwg_entity_raster_image_set_size(data, text, 800.0, 600.0) !=
      DWG_ETYPE) {
    exit_code = 26;
    goto cleanup;
  }

cleanup:
  dwg_data_destroy(data);
  return exit_code;
}
