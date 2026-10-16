#include "dwg_api.h"

static void callback(int code, const char *message, void *user_data) {
    (void)code;
    (void)message;
    (void)user_data;
}

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != DWG_OK || data == NULL) return 1;
    if (dwg_set_notification_callback(data, callback, NULL) != DWG_OK) return 2;
    if (dwg_set_notification_callback(data, NULL, (void *)1) != DWG_OK) return 3;

    DwgObject *line = NULL;
    if (dwg_entity_line_create(data, 0, 1, 2, 3, 4, 5, &line) != DWG_OK) return 4;
    if (line == NULL) return 5;

    int32_t entity_count = -1;
    int32_t object_count = -1;
    if (dwg_data_entity_count(data, &entity_count) != DWG_OK || entity_count != 1) return 6;
    if (dwg_data_object_count(data, &object_count) != DWG_OK || object_count <= 0) return 7;

    int32_t error = DWG_EINVAL;
    uint64_t handle = dwg_object_get_handle(line);
    if (handle == 0) return 8;
    if (dwg_object_by_handle(data, handle, &error) != line || error != DWG_OK) return 9;
    if (dwg_object_by_handle(data, UINT64_MAX, &error) != NULL || error != DWG_ENOTFOUND) return 10;

    dwg_data_destroy(data);
    return 0;
}
