#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
    DWG_OK = 0,
    DWG_ETYPE = 8,
    DWG_OBJECT_SOLID3D = 32
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *object);
extern int dwg_object_get_type(const DwgObject *object);
extern int dwg_entity_acis_is_binary(const DwgObject *object, int *binary);
extern int dwg_entity_acis_get_sat(const DwgObject *object, char *buf,
                                   size_t cap, size_t *needed);
extern int dwg_entity_acis_get_sab(const DwgObject *object, uint8_t *buf,
                                   size_t cap, size_t *needed);
extern int dwg_entity_acis_set_sat(DwgData *data, const DwgObject *object,
                                   const char *sat);
extern int dwg_entity_acis_set_sab(DwgData *data, const DwgObject *object,
                                   const uint8_t *sab, size_t len);

int main(int argc, char **argv) {
    if (argc != 2) return 1;
    DwgData *data = NULL;
    if (dwg_data_create(&data) != DWG_OK || data == NULL) return 2;
    if (dwg_read_file(argv[1], data) != DWG_OK) return 3;

    int error = -1;
    const DwgObject *object = dwg_data_first_object(data, &error);
    while (object != NULL && dwg_object_get_type(object) != DWG_OBJECT_SOLID3D) {
        object = dwg_object_next(object);
    }
    if (object == NULL) return 4;

    int binary = -1;
    size_t needed = 0;
    if (dwg_entity_acis_is_binary(object, &binary) != DWG_OK) return 5;
    if (binary != 0 && binary != 1) return 6;
    if (binary == 0) {
        if (dwg_entity_acis_get_sat(object, NULL, 0, &needed) != DWG_OK || needed <= 1) return 7;
    } else {
        if (dwg_entity_acis_get_sab(object, NULL, 0, &needed) != DWG_OK || needed == 0) return 8;
    }

    const char *sat = "700 0 1 0\nC smoke replacement\n";
    if (dwg_entity_acis_set_sat(data, object, sat) != DWG_OK) return 9;
    if (dwg_entity_acis_is_binary(object, &binary) != DWG_OK || binary != 0) return 10;
    char sat_out[64];
    if (dwg_entity_acis_get_sat(object, sat_out, sizeof sat_out, &needed) != DWG_OK) return 11;
    if (strcmp(sat_out, sat) != 0) return 12;
    if (dwg_entity_acis_get_sab(object, NULL, 0, &needed) != DWG_ETYPE) return 13;

    const uint8_t sab[] = {0xA1, 0xB2, 0xC3};
    if (dwg_entity_acis_set_sab(data, object, sab, sizeof sab) != DWG_OK) return 14;
    if (dwg_entity_acis_is_binary(object, &binary) != DWG_OK || binary != 1) return 15;
    uint8_t sab_out[sizeof sab];
    if (dwg_entity_acis_get_sab(object, sab_out, sizeof sab_out, &needed) != DWG_OK ||
        needed != sizeof sab || memcmp(sab_out, sab, sizeof sab) != 0) return 16;

    dwg_data_destroy(data);
    return 0;
}
