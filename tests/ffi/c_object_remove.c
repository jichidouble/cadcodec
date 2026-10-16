#include <stdint.h>
#include <stddef.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
    DWG_OK = 0,
    DWG_EINVAL = 1,
    DWG_ENOTFOUND = 6,
    DWG_OBJECT_UNKNOWN = 0,
    DWG_OBJECT_LINE = 1,
    DWG_OBJECT_CIRCLE = 2
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *object);
extern int dwg_object_get_type(const DwgObject *object);
extern uint64_t dwg_object_get_handle(const DwgObject *object);
extern int dwg_object_remove(DwgData *data, const DwgObject *object);

int main(int argc, char **argv) {
    if (argc != 2) return 1;

    DwgData *data = NULL;
    DwgData *foreign_data = NULL;
    if (dwg_data_create(&data) != DWG_OK || data == NULL) return 2;
    if (dwg_data_create(&foreign_data) != DWG_OK || foreign_data == NULL) return 3;
    if (dwg_read_file(argv[1], data) != DWG_OK) return 4;

    int error = -1;
    const DwgObject *line = dwg_data_first_object(data, &error);
    if (error != DWG_OK || line == NULL || dwg_object_get_type(line) != DWG_OBJECT_LINE) return 5;
    const DwgObject *circle = dwg_object_next(line);
    if (circle == NULL || dwg_object_get_type(circle) != DWG_OBJECT_CIRCLE) return 6;
    uint64_t circle_handle = dwg_object_get_handle(circle);

    if (dwg_object_remove(data, line) != DWG_OK) return 7;
    if (dwg_object_get_type(line) != DWG_OBJECT_UNKNOWN || dwg_object_get_handle(line) != 0) return 8;
    if (dwg_object_remove(data, line) != DWG_ENOTFOUND) return 9;
    if (dwg_object_remove(data, NULL) != DWG_EINVAL) return 10;

    const DwgObject *first_after = dwg_data_first_object(data, &error);
    if (error != DWG_OK || first_after != circle) return 11;
    if (dwg_object_get_handle(circle) != circle_handle) return 12;

    const DwgObject *foreign = dwg_data_first_object(foreign_data, &error);
    if (foreign == NULL || dwg_object_remove(data, foreign) != DWG_ENOTFOUND) return 13;

    dwg_data_destroy(foreign_data);
    dwg_data_destroy(data);
    return 0;
}
