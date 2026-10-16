#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct DwgData DwgData;
typedef void (*DwgNotificationCallback)(int, const char *, void *);

enum {
    DWG_OK = 0,
    DWG_EINVAL = 1,
    DWG_NOTIFICATION_NOT_IMPLEMENTED = 1
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dxf_read_file(const char *path, DwgData *data);
extern int dwg_set_notification_callback(DwgData *data,
                                          DwgNotificationCallback callback,
                                          void *user_data);

static int callback_count;
static int callback_code;
static int callback_message_valid;
static void *callback_user;

static void record_notification(int code, const char *message, void *user_data) {
    callback_count += 1;
    callback_code = code;
    callback_message_valid = message != NULL && message[0] != '\0';
    callback_user = user_data;
}

int main(int argc, char **argv) {
    if (argc != 2) return 1;
    DwgData *data = NULL;
    unsigned char token = 0xC2;
    if (dwg_data_create(&data) != DWG_OK || data == NULL) return 2;
    if (dwg_set_notification_callback(data, record_notification, &token) != DWG_OK) return 3;
    if (dxf_read_file(argv[1], data) != DWG_OK) return 4;
    if (callback_count == 0) return 5;
    if (callback_code != DWG_NOTIFICATION_NOT_IMPLEMENTED) return 6;
    if (!callback_message_valid) return 7;
    if (callback_user != &token) return 8;

    callback_count = 0;
    if (dwg_set_notification_callback(data, NULL, NULL) != DWG_OK) return 6;
    if (dxf_read_file(argv[1], data) != DWG_OK) return 7;
    if (callback_count != 0) return 8;

    dwg_data_destroy(data);
    return 0;
}
