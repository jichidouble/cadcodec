#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;
enum {
    DWG_OBJECT_UNKNOWN = 0,
    DWG_OBJECT_LINE = 1,
    DWG_OBJECT_CIRCLE = 2,
    DWG_OBJECT_ARC = 3,
    DWG_OBJECT_TEXT = 4,
    DWG_OBJECT_POINT = 5,
    DWG_OBJECT_OTHER = 1000,
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);
extern uint64_t dwg_object_get_handle(const DwgObject *obj);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;

    int err = 0;
    const DwgObject *obj = dwg_data_first_object(data, &err);
    if (err != 0) return 2;
    if (obj == NULL) return 3; /* fresh document carries default objects */

    /* Walk the whole object graph; every object must carry a type and handle. */
    size_t count = 0;
    while (obj != NULL) {
        int type = dwg_object_get_type(obj);
        uint64_t handle = dwg_object_get_handle(obj);
        if (type == DWG_OBJECT_UNKNOWN) return 4;
        if (handle == 0) return 5;
        count++;
        obj = dwg_object_next(obj);
    }
    if (count == 0) return 6;

    /* NULL handling: bad pointer returns UNKNOWN type, NULL next returns NULL. */
    if (dwg_object_get_type(NULL) != DWG_OBJECT_UNKNOWN) return 7;
    if (dwg_object_next(NULL) != NULL) return 8;

    dwg_data_destroy(data);
    return 0;
}
