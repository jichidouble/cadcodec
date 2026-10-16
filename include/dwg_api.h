#ifndef ACADRUST_DWG_API_H
#define ACADRUST_DWG_API_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct DwgData DwgData;
typedef void (*DwgNotificationCallback)(int32_t notification_type,
                                        const char *message,
                                        void *user_data);

/**
 * Install or clear the diagnostic callback for a document.
 */
int32_t dwg_set_notification_callback(DwgData *data,
                                      DwgNotificationCallback callback,
                                      void *user_data);

#ifdef __cplusplus
}  // extern "C"
#endif


/**
 * Success.
 */
#define DWG_OK 0

/**
 * Invalid argument or NULL pointer.
 */
#define DWG_EINVAL 1

/**
 * Allocation failure.
 */
#define DWG_ENOMEM 2

/**
 * Read failure.
 */
#define DWG_EREAD 3

/**
 * Write failure.
 */
#define DWG_EWRITE 4

/**
 * Unsupported version.
 */
#define DWG_EVERSION 5

/**
 * Object/entity not found.
 */
#define DWG_ENOTFOUND 6

/**
 * Buffer too small (`needed` carries the required size).
 */
#define DWG_EBUFFER 7

/**
 * Type mismatch.
 */
#define DWG_ETYPE 8

/**
 * Notification code for an unsupported implementation.
 */
#define DWG_NOTIFICATION_NOT_IMPLEMENTED 1

/**
 * Notification code for a feature unsupported in the current context.
 */
#define DWG_NOTIFICATION_NOT_SUPPORTED 2

/**
 * Notification code for a recoverable warning.
 */
#define DWG_NOTIFICATION_WARNING 3

/**
 * Notification code for a recoverable error.
 */
#define DWG_NOTIFICATION_ERROR 4

/**
 * String XRECORD value tag.
 */
#define DWG_XRECORD_STRING 1

/**
 * Double XRECORD value tag.
 */
#define DWG_XRECORD_DOUBLE 2

/**
 * Signed 16-bit integer XRECORD value tag.
 */
#define DWG_XRECORD_INT16 3

/**
 * Signed 32-bit integer XRECORD value tag.
 */
#define DWG_XRECORD_INT32 4

/**
 * Signed 64-bit integer XRECORD value tag.
 */
#define DWG_XRECORD_INT64 5

/**
 * Unsigned byte XRECORD value tag.
 */
#define DWG_XRECORD_BYTE 6

/**
 * Boolean XRECORD value tag.
 */
#define DWG_XRECORD_BOOL 7

/**
 * Handle or ObjectId XRECORD value tag.
 */
#define DWG_XRECORD_HANDLE 8

/**
 * Three-dimensional point XRECORD value tag.
 */
#define DWG_XRECORD_POINT3D 9

/**
 * Binary chunk XRECORD value tag.
 */
#define DWG_XRECORD_CHUNK 10

/**
 * String XData value tag.
 */
#define DWG_XDATA_STRING 1

/**
 * Control-string XData value tag.
 */
#define DWG_XDATA_CONTROL_STRING 2

/**
 * Layer-name XData value tag.
 */
#define DWG_XDATA_LAYER_NAME 3

/**
 * Binary XData value tag.
 */
#define DWG_XDATA_BINARY 4

/**
 * Handle XData value tag.
 */
#define DWG_XDATA_HANDLE 5

/**
 * Three-dimensional point XData value tag.
 */
#define DWG_XDATA_POINT3D 6

/**
 * Three-dimensional position XData value tag.
 */
#define DWG_XDATA_POSITION3D 7

/**
 * Three-dimensional displacement XData value tag.
 */
#define DWG_XDATA_DISPLACEMENT3D 8

/**
 * Three-dimensional direction XData value tag.
 */
#define DWG_XDATA_DIRECTION3D 9

/**
 * Real XData value tag.
 */
#define DWG_XDATA_REAL 10

/**
 * Distance XData value tag.
 */
#define DWG_XDATA_DISTANCE 11

/**
 * Scale-factor XData value tag.
 */
#define DWG_XDATA_SCALE_FACTOR 12

/**
 * Signed 16-bit integer XData value tag.
 */
#define DWG_XDATA_INTEGER16 13

/**
 * Signed 32-bit integer XData value tag.
 */
#define DWG_XDATA_INTEGER32 14

/**
 * Object type as seen by C consumers. Extended in later phases.
 */
enum DwgObjectType
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
  Unknown = 0,
  Line = 1,
  Circle = 2,
  Arc = 3,
  Text = 4,
  Point = 5,
  Ellipse = 6,
  MText = 7,
  Spline = 8,
  LwPolyline = 9,
  Ray = 10,
  XLine = 11,
  Solid = 12,
  Face3D = 13,
  Insert = 14,
  Polyline2D = 15,
  Polyline3D = 16,
  Mesh = 17,
  Viewport = 18,
  PolyfaceMesh = 19,
  Hatch = 20,
  Tolerance = 21,
  Leader = 22,
  MLine = 23,
  AttributeDefinition = 24,
  Shape = 25,
  Helix = 26,
  Dimension = 27,
  RasterImage = 28,
  Underlay = 29,
  Light = 30,
  Wipeout = 31,
  Solid3D = 32,
  Region = 33,
  Body = 34,
  Surface = 35,
  Other = 1000,
};
#ifndef __cplusplus
typedef int32_t DwgObjectType;
#endif // __cplusplus

/**
 * Which symbol table a generic call targets.
 */
enum DwgTableKind
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
  Layer = 0,
  Block = 1,
  Style = 2,
  DimStyle = 3,
};
#ifndef __cplusplus
typedef int32_t DwgTableKind;
#endif // __cplusplus

/**
 * Opaque reference to an object inside a document. Owned by the document;
 * never freed by the caller. Stable until the document is destroyed.
 */
typedef struct DwgObject {
  uint8_t _priv[0];
} DwgObject;

/**
 * Opaque document handle. The document is the sole ownership root. Created
 * by `dwg_data_create`, destroyed by `dwg_data_destroy`.
 */
typedef struct DwgData {
  uint8_t _priv[0];
} DwgData;

/**
 * A typed XRECORD value exchanged through the C ABI.
 */
typedef struct DwgXrecordValue {
  /**
   * Payload discriminator; one of the `DWG_XRECORD_*` constants.
   */
  uint32_t tag;
  /**
   * Reserved for ABI evolution; mutation callers must set this to zero.
   */
  uint32_t reserved;
  /**
   * Integer payload for Int16, Int32, Int64, Byte, and Bool values.
   */
  int64_t integer;
  /**
   * Floating-point payload for Double values.
   */
  double real;
  /**
   * Three-dimensional payload for Point3D values.
   */
  double point[3];
  /**
   * Document-owned handle payload for Handle and ObjectId values.
   */
  uint64_t handle;
  /**
   * Caller-owned string or chunk storage.
   */
  void *data;
  /**
   * Input length or output capacity/required length for `data`.
   */
  size_t data_len;
} DwgXrecordValue;

/**
 * A typed XData value exchanged through the C ABI.
 */
typedef struct DwgXdataValue {
  /**
   * Payload discriminator; one of the `DWG_XDATA_*` constants.
   */
  uint32_t tag;
  /**
   * Reserved for ABI evolution; mutation callers must set this to zero.
   */
  uint32_t reserved;
  /**
   * Integer payload for Integer16 and Integer32 values.
   */
  int64_t integer;
  /**
   * Floating-point payload for real, distance, and scale-factor values.
   */
  double real;
  /**
   * Three-dimensional point, position, displacement, or direction payload.
   */
  double point[3];
  /**
   * Document-owned handle payload.
   */
  uint64_t handle;
  /**
   * Caller-owned string or binary storage.
   */
  void *data;
  /**
   * Input length or output capacity/required length for `data`.
   */
  size_t data_len;
} DwgXdataValue;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Copy the current thread's last error message into `buf` (caller-owned).
 * `needed` receives the total bytes required including the NUL terminator.
 *
 * Return value is the error code that was stored (`DWG_OK` when none is
 * pending), matching `dwg_last_error_code`; callers that only need the code
 * may use `dwg_last_error_code` directly, callers that need the message use
 * this function. The slot is sticky: it is only replaced by the next failed
 * operation and cleared by the next successful one.
 */
int32_t dwg_last_error(int8_t *buf, size_t cap, size_t *needed);

/**
 * Return the current thread's last error code, or `DWG_OK` when none.
 */
int32_t dwg_last_error_code(void);

/**
 * Return the library version as a NUL-terminated static string.
 */
const char *dwg_version(void);

/**
 * Parse a version string like `AC1032` and return its numeric DXF version,
 * or 0 when unrecognised.
 */
int32_t dwg_parse_version(const char *text);

/**
 * Return whether an ACIS entity stores binary SAB data (`1`) or SAT text (`0`).
 */
int32_t dwg_entity_acis_is_binary(const struct DwgObject *obj, int32_t *binary);

/**
 * Copy SAT text, including its terminating NUL, into caller-owned storage.
 */
int32_t dwg_entity_acis_get_sat(const struct DwgObject *obj, char *buf, size_t cap, size_t *needed);

/**
 * Copy SAB bytes into caller-owned storage.
 */
int32_t dwg_entity_acis_get_sab(const struct DwgObject *obj,
                                uint8_t *buf,
                                size_t cap,
                                size_t *needed);

/**
 * Replace an ACIS entity's payload with copied SAT text.
 */
int32_t dwg_entity_acis_set_sat(struct DwgData *data, const struct DwgObject *obj, const char *sat);

/**
 * Replace an ACIS entity's payload with copied SAB bytes.
 */
int32_t dwg_entity_acis_set_sab(struct DwgData *data,
                                const struct DwgObject *obj,
                                const uint8_t *sab,
                                size_t len);

/**
 * Create a new empty document. On success `*out` receives a document handle
 * owned by the caller (release with `dwg_data_destroy`).
 */
int32_t dwg_data_create(struct DwgData **out);

/**
 * Write the number of live document entities to `count`.
 */
int32_t dwg_data_entity_count(const struct DwgData *data, int32_t *count);

/**
 * Write the number of live entities and objects to `count`.
 */
int32_t dwg_data_object_count(const struct DwgData *data, int32_t *count);

/**
 * Destroy a document previously created by `dwg_data_create`. All object
 * references obtained from this document become invalid.
 */
void dwg_data_destroy(struct DwgData *data);

/**
 * Copy the document's DWG version string (e.g. "AC1032"; a fresh document
 * created by `dwg_data_create` is AC1032) into `buf` (caller-owned).
 * `needed` receives the total bytes required including NUL.
 */
int32_t dwg_data_version(const struct DwgData *data, int8_t *buf, size_t cap, size_t *needed);

/**
 * Read a DWG file into `data`, replacing its contents.
 */
int32_t dwg_read_file(const char *path, struct DwgData *data);

/**
 * Write `data` to a DWG file.
 */
int32_t dwg_write_file(const char *path, const struct DwgData *data);

/**
 * Read a DXF file into `data`, replacing its contents.
 */
int32_t dxf_read_file(const char *path, struct DwgData *data);

/**
 * Write `data` to a DXF file.
 */
int32_t dxf_write_file(const char *path, const struct DwgData *data);

/**
 * Read a DWG byte sequence into `data`, replacing its contents only after a
 * complete parse succeeds.
 *
 * # Safety
 * `input` must point to `input_len` readable bytes and `data` must be a valid
 * mutable document handle created by `dwg_data_create`.
 */
int32_t dwg_read_bytes(const uint8_t *input, size_t input_len, struct DwgData *data);

/**
 * Write a DWG document to a newly allocated byte buffer.
 *
 * # Safety
 * `data` must be a valid document handle. `output` and `output_len` must be
 * valid writable pointers. The returned buffer must be released with
 * `dwg_free_buffer`.
 */
int32_t dwg_write_bytes(const struct DwgData *data, uint8_t **output, size_t *output_len);

/**
 * Release a byte buffer returned by `dwg_write_bytes`.
 *
 * # Safety
 * `buffer` must be NULL or a pointer returned by a successful
 * `dwg_write_bytes` call that has not already been released.
 */
void dwg_free_buffer(void *buffer);

/**
 * Remove an object from `data`.
 *
 * The object pointer remains a safe, invalid arena token until
 * `dwg_data_destroy`; all other active object pointers retain their address.
 */
int32_t dwg_object_remove(struct DwgData *data, const struct DwgObject *obj);

/**
 * Return the first object of the document, or NULL when the document is
 * empty. `*error` receives `DWG_OK` on success, `DWG_EINVAL` on bad args.
 */
const struct DwgObject *dwg_data_first_object(const struct DwgData *data, int32_t *error);

/**
 * Find a live document object by its 64-bit handle. The returned pointer is
 * borrowed from `data` and must not be freed by the caller.
 */
const struct DwgObject *dwg_object_by_handle(const struct DwgData *data,
                                             uint64_t handle,
                                             int32_t *error);

/**
 * Return the next object after `obj`, or NULL at the end of the document.
 */
const struct DwgObject *dwg_object_next(const struct DwgObject *obj);

/**
 * Return the object type as a `DwgObjectType` value, or
 * `DWG_OBJECT_UNKNOWN` for a NULL pointer.
 */
int32_t dwg_object_get_type(const struct DwgObject *obj);

/**
 * Return the object's handle value (0 for NULL).
 */
uint64_t dwg_object_get_handle(const struct DwgObject *obj);

/**
 * Copy the TEXT entity's value into a caller-owned buffer.
 */
int32_t dwg_entity_text_get_value(const struct DwgObject *obj,
                                  int8_t *buf,
                                  size_t cap,
                                  size_t *needed);

/**
 * Copy the MTEXT entity's value into a caller-owned buffer.
 */
int32_t dwg_entity_mtext_get_value(const struct DwgObject *obj,
                                   int8_t *buf,
                                   size_t cap,
                                   size_t *needed);

/**
 * Copy the entity's layer name into a caller-owned buffer.
 */
int32_t dwg_entity_get_layer(const struct DwgObject *obj, int8_t *buf, size_t cap, size_t *needed);

/**
 * Read the entity's color (RGB >= 0, ACI index negative).
 */
int32_t dwg_entity_get_color(const struct DwgObject *obj, int32_t *color);

/**
 * Copy the entity's linetype name into a caller-owned buffer.
 */
int32_t dwg_entity_get_linetype(const struct DwgObject *obj,
                                int8_t *buf,
                                size_t cap,
                                size_t *needed);

/**
 * Read the entity's linetype scale.
 */
int32_t dwg_entity_get_linetype_scale(const struct DwgObject *obj, double *scale);

/**
 * Read one of the four SOLID corners (`corner` 1..=4).
 */
int32_t dwg_entity_solid_get_corner(const struct DwgObject *obj,
                                    int32_t corner,
                                    double *x,
                                    double *y,
                                    double *z);

/**
 * Read one of the four 3DFACE corners (`corner` 1..=4).
 */
int32_t dwg_entity_face3d_get_corner(const struct DwgObject *obj,
                                     int32_t corner,
                                     double *x,
                                     double *y,
                                     double *z);

/**
 * Read the SPLINE degree.
 */
int32_t dwg_entity_spline_get_degree(const struct DwgObject *obj, int32_t *degree);

/**
 * Read the SPLINE control point count.
 */
int32_t dwg_entity_spline_get_control_point_count(const struct DwgObject *obj, int32_t *count);

/**
 * Read one SPLINE control point by index.
 */
int32_t dwg_entity_spline_get_control_point(const struct DwgObject *obj,
                                            int32_t index,
                                            double *x,
                                            double *y,
                                            double *z);

/**
 * Read the LWPOLYLINE vertex count.
 */
int32_t dwg_entity_lwpolyline_get_vertex_count(const struct DwgObject *obj, int32_t *count);

/**
 * Read one LWPOLYLINE vertex (2D) by index.
 */
int32_t dwg_entity_lwpolyline_get_vertex(const struct DwgObject *obj,
                                         int32_t index,
                                         double *x,
                                         double *y);

/**
 * Copy the INSERT block name into a caller-owned buffer.
 */
int32_t dwg_entity_insert_get_block_name(const struct DwgObject *obj,
                                         int8_t *buf,
                                         size_t cap,
                                         size_t *needed);

/**
 * Read the INSERT column/row counts.
 */
int32_t dwg_entity_insert_get_array(const struct DwgObject *obj, uint16_t *columns, uint16_t *rows);

/**
 * Read the MESH vertex count / a vertex by index (vertices are Vector3).
 */
int32_t dwg_entity_mesh_get_vertex_count(const struct DwgObject *obj, int32_t *count);

int32_t dwg_entity_mesh_get_vertex(const struct DwgObject *obj,
                                   int32_t index,
                                   double *x,
                                   double *y,
                                   double *z);

/**
 * Read the MESH face count and the number of vertices of one face.
 */
int32_t dwg_entity_mesh_get_face_count(const struct DwgObject *obj, int32_t *count);

int32_t dwg_entity_mesh_get_face_vertex_count(const struct DwgObject *obj,
                                              int32_t face,
                                              int32_t *count);

/**
 * Read one zero-based vertex index from a MESH face.
 */
int32_t dwg_entity_mesh_get_face_vertex(const struct DwgObject *obj,
                                        int32_t face,
                                        int32_t slot,
                                        uint32_t *vertex);

/**
 * Copy the DIMENSION style name into a caller-owned buffer.
 */
int32_t dwg_entity_dimension_get_style_name(const struct DwgObject *obj,
                                            int8_t *buf,
                                            size_t cap,
                                            size_t *needed);

/**
 * Read the DIMENSION definition point (WCS).
 */
int32_t dwg_entity_dimension_get_definition_point(const struct DwgObject *obj,
                                                  double *x,
                                                  double *y,
                                                  double *z);

/**
 * Read the DIMENSION text middle point (WCS).
 */
int32_t dwg_entity_dimension_get_text_middle_point(const struct DwgObject *obj,
                                                   double *x,
                                                   double *y,
                                                   double *z);

/**
 * Read the DIMENSION subtype (1 Aligned, 2 Linear, 3 Radius, 4 Diameter,
 * 5 Angular2Ln, 6 Angular3Pt, 7 Ordinate, 8 Arc, 9 LargeRadial).
 */
int32_t dwg_entity_dimension_get_subtype(const struct DwgObject *obj, int32_t *subtype);

/**
 * Read the DIMENSION measurement value.
 */
int32_t dwg_entity_dimension_get_measurement(const struct DwgObject *obj, double *value);

/**
 * Read the RASTERIMAGE pixel size (width/height).
 */
int32_t dwg_entity_raster_image_get_size(const struct DwgObject *obj,
                                         double *width,
                                         double *height);

/**
 * Copy the HATCH pattern name into a caller-owned buffer.
 */
int32_t dwg_entity_hatch_get_pattern_name(const struct DwgObject *obj,
                                          int8_t *buf,
                                          size_t cap,
                                          size_t *needed);

/**
 * Create a LINE entity in model space.
 */
int32_t dwg_entity_line_create(struct DwgData *data,
                               double x1,
                               double y1,
                               double z1,
                               double x2,
                               double y2,
                               double z2,
                               struct DwgObject **out);

/**
 * Create a CIRCLE entity in model space.
 */
int32_t dwg_entity_circle_create(struct DwgData *data,
                                 double x,
                                 double y,
                                 double z,
                                 double radius,
                                 struct DwgObject **out);

/**
 * Create an ARC entity in model space.
 */
int32_t dwg_entity_arc_create(struct DwgData *data,
                              double x,
                              double y,
                              double z,
                              double radius,
                              double start_angle,
                              double end_angle,
                              struct DwgObject **out);

/**
 * Create a POINT entity in model space.
 */
int32_t dwg_entity_point_create(struct DwgData *data,
                                double x,
                                double y,
                                double z,
                                struct DwgObject **out);

/**
 * Set one LWPOLYLINE vertex location while preserving its segment metadata.
 */
int32_t dwg_entity_lwpolyline_set_vertex(struct DwgData *data,
                                         const struct DwgObject *obj,
                                         int32_t index,
                                         double x,
                                         double y);

/**
 * Append one LWPOLYLINE vertex.
 */
int32_t dwg_entity_lwpolyline_append_vertex(struct DwgData *data,
                                            const struct DwgObject *obj,
                                            double x,
                                            double y);

/**
 * Remove one LWPOLYLINE vertex.
 */
int32_t dwg_entity_lwpolyline_remove_vertex(struct DwgData *data,
                                            const struct DwgObject *obj,
                                            int32_t index);

/**
 * Clear all LWPOLYLINE vertices.
 */
int32_t dwg_entity_lwpolyline_clear_vertices(struct DwgData *data, const struct DwgObject *obj);

/**
 * Set one POLYFACE MESH vertex location while preserving its metadata.
 */
int32_t dwg_entity_polyface_mesh_set_vertex(struct DwgData *data,
                                            const struct DwgObject *obj,
                                            int32_t index,
                                            double x,
                                            double y,
                                            double z);

/**
 * Append one POLYFACE MESH vertex.
 */
int32_t dwg_entity_polyface_mesh_append_vertex(struct DwgData *data,
                                               const struct DwgObject *obj,
                                               double x,
                                               double y,
                                               double z);

/**
 * Remove one POLYFACE MESH vertex and remap subsequent face references.
 */
int32_t dwg_entity_polyface_mesh_remove_vertex(struct DwgData *data,
                                               const struct DwgObject *obj,
                                               int32_t index);

/**
 * Clear all POLYFACE MESH vertices when no face records reference them.
 */
int32_t dwg_entity_polyface_mesh_clear_vertices(struct DwgData *data, const struct DwgObject *obj);

/**
 * Set one MESH vertex location.
 */
int32_t dwg_entity_mesh_set_vertex(struct DwgData *data,
                                   const struct DwgObject *obj,
                                   int32_t index,
                                   double x,
                                   double y,
                                   double z);

/**
 * Append one MESH vertex.
 */
int32_t dwg_entity_mesh_append_vertex(struct DwgData *data,
                                      const struct DwgObject *obj,
                                      double x,
                                      double y,
                                      double z);

/**
 * Remove an unreferenced MESH vertex and remap subsequent topology indices.
 */
int32_t dwg_entity_mesh_remove_vertex(struct DwgData *data,
                                      const struct DwgObject *obj,
                                      int32_t index);

/**
 * Clear all MESH vertices and edges when no faces remain.
 */
int32_t dwg_entity_mesh_clear_vertices(struct DwgData *data, const struct DwgObject *obj);

/**
 * Replace one MESH face vertex index and recompute edges.
 */
int32_t dwg_entity_mesh_set_face_vertex(struct DwgData *data,
                                        const struct DwgObject *obj,
                                        int32_t face,
                                        int32_t slot,
                                        uint32_t vertex);

/**
 * Replace a MESH face with indices supplied by the caller.
 */
int32_t dwg_entity_mesh_set_face(struct DwgData *data,
                                 const struct DwgObject *obj,
                                 int32_t face,
                                 const uint32_t *indices,
                                 size_t count);

/**
 * Append a MESH face with indices supplied by the caller.
 */
int32_t dwg_entity_mesh_append_face(struct DwgData *data,
                                    const struct DwgObject *obj,
                                    const uint32_t *indices,
                                    size_t count);

/**
 * Remove one MESH face and recompute edges.
 */
int32_t dwg_entity_mesh_remove_face(struct DwgData *data,
                                    const struct DwgObject *obj,
                                    int32_t face);

/**
 * Clear all MESH faces and derived edges.
 */
int32_t dwg_entity_mesh_clear_faces(struct DwgData *data, const struct DwgObject *obj);

/**
 * Set an entity's layer name.
 *
 * # Safety
 *
 * `data` must be a valid document pointer, `obj` must be a valid entity object from that
 * document, and `value` must point to a valid NUL-terminated string.
 */
int32_t dwg_entity_set_layer(struct DwgData *data,
                             const struct DwgObject *obj,
                             const int8_t *value);

/**
 * Set an entity's color using the C ABI color encoding.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid entity object from that
 * document.
 */
int32_t dwg_entity_set_color(struct DwgData *data, const struct DwgObject *obj, int32_t value);

/**
 * Set an entity's linetype name.
 *
 * # Safety
 *
 * `data` must be a valid document pointer, `obj` must be a valid entity object from that
 * document, and `value` must point to a valid NUL-terminated string.
 */
int32_t dwg_entity_set_linetype(struct DwgData *data,
                                const struct DwgObject *obj,
                                const int8_t *value);

/**
 * Set an entity's positive, finite linetype scale.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid entity object from that
 * document.
 */
int32_t dwg_entity_set_linetype_scale(struct DwgData *data,
                                      const struct DwgObject *obj,
                                      double value);

/**
 * Set an INSERT entity's block name.
 *
 * # Safety
 *
 * `data` must be a valid document pointer, `obj` must be a valid INSERT object from that
 * document, and `value` must point to a valid non-empty NUL-terminated string.
 */
int32_t dwg_entity_insert_set_block_name(struct DwgData *data,
                                         const struct DwgObject *obj,
                                         const int8_t *value);

/**
 * Set an INSERT entity's normalized normal vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
 * document.
 */
int32_t dwg_entity_insert_set_normal(struct DwgData *data,
                                     const struct DwgObject *obj,
                                     double x,
                                     double y,
                                     double z);

/**
 * Set an INSERT entity's array dimensions and spacing.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
 * document.
 */
int32_t dwg_entity_insert_set_array(struct DwgData *data,
                                    const struct DwgObject *obj,
                                    uint16_t columns,
                                    uint16_t rows,
                                    double column_spacing,
                                    double row_spacing);

/**
 * Set a HATCH entity's pattern name.
 *
 * # Safety
 *
 * `data` must be a valid document pointer, `obj` must be a valid HATCH object from that
 * document, and `value` must point to a valid NUL-terminated string.
 */
int32_t dwg_entity_hatch_set_pattern_name(struct DwgData *data,
                                          const struct DwgObject *obj,
                                          const int8_t *value);

/**
 * Set a RAY entity's normalized direction vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid RAY object from that
 * document.
 */
int32_t dwg_entity_ray_set_direction(struct DwgData *data,
                                     const struct DwgObject *obj,
                                     double x,
                                     double y,
                                     double z);

/**
 * Set an XLINE entity's normalized direction vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid XLINE object from that
 * document.
 */
int32_t dwg_entity_xline_set_direction(struct DwgData *data,
                                       const struct DwgObject *obj,
                                       double x,
                                       double y,
                                       double z);

/**
 * Set one of a SOLID entity's four corners (`corner` 1..=4).
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SOLID object from that
 * document.
 */
int32_t dwg_entity_solid_set_corner(struct DwgData *data,
                                    const struct DwgObject *obj,
                                    int32_t corner,
                                    double x,
                                    double y,
                                    double z);

/**
 * Set one of a 3DFACE entity's four corners (`corner` 1..=4).
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid 3DFACE object from that
 * document.
 */
int32_t dwg_entity_face3d_set_corner(struct DwgData *data,
                                     const struct DwgObject *obj,
                                     int32_t corner,
                                     double x,
                                     double y,
                                     double z);

/**
 * Set a DIMENSION entity's style name.
 *
 * # Safety
 *
 * `data` must be a valid document pointer, `obj` must be a valid DIMENSION object from that
 * document, and `value` must point to a valid NUL-terminated string.
 */
int32_t dwg_entity_dimension_set_style_name(struct DwgData *data,
                                            const struct DwgObject *obj,
                                            const int8_t *value);

/**
 * Set a DIMENSION entity's definition point.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
 * document.
 */
int32_t dwg_entity_dimension_set_definition_point(struct DwgData *data,
                                                  const struct DwgObject *obj,
                                                  double x,
                                                  double y,
                                                  double z);

/**
 * Set a DIMENSION entity's text middle point.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
 * document.
 */
int32_t dwg_entity_dimension_set_text_middle_point(struct DwgData *data,
                                                   const struct DwgObject *obj,
                                                   double x,
                                                   double y,
                                                   double z);

/**
 * Set a RASTERIMAGE entity's pixel width and height atomically.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid RASTERIMAGE object from that
 * document.
 */
int32_t dwg_entity_raster_image_set_size(struct DwgData *data,
                                         const struct DwgObject *obj,
                                         double width,
                                         double height);

/**
 * Set a SPLINE entity's degree and regenerate its clamped knot vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
 * document.
 */
int32_t dwg_entity_spline_set_degree(struct DwgData *data,
                                     const struct DwgObject *obj,
                                     int32_t degree);

/**
 * Replace one zero-based SPLINE control point and regenerate its clamped knot vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
 * document.
 */
int32_t dwg_entity_spline_set_control_point(struct DwgData *data,
                                            const struct DwgObject *obj,
                                            int32_t index,
                                            double x,
                                            double y,
                                            double z);

/**
 * Append a SPLINE control point and regenerate its clamped knot vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
 * document.
 */
int32_t dwg_entity_spline_append_control_point(struct DwgData *data,
                                               const struct DwgObject *obj,
                                               double x,
                                               double y,
                                               double z);

/**
 * Remove one zero-based SPLINE control point and regenerate its clamped knot vector.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
 * document.
 */
int32_t dwg_entity_spline_remove_control_point(struct DwgData *data,
                                               const struct DwgObject *obj,
                                               int32_t index);

/**
 * Clear a SPLINE entity's control points, knots, and weights.
 *
 * # Safety
 *
 * `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
 * document.
 */
int32_t dwg_entity_spline_clear_control_points(struct DwgData *data, const struct DwgObject *obj);

/**
 * Number of layers in the document.
 */
int32_t dwg_layer_table_count(const struct DwgData *data, int32_t *count);

/**
 * Copy the name of the layer at `index` (iteration order).
 */
int32_t dwg_layer_get_name(const struct DwgData *data,
                           int32_t index,
                           int8_t *buf,
                           size_t cap,
                           size_t *needed);

/**
 * Read the layer color at `index` as an ARGB int32.
 */
int32_t dwg_layer_get_color(const struct DwgData *data, int32_t index, int32_t *color);

/**
 * Read the layer handle at `index`.
 */
int32_t dwg_layer_get_handle(const struct DwgData *data, int32_t index, uint64_t *handle);

/**
 * Set the layer color at `index`.
 */
int32_t dwg_layer_set_color(struct DwgData *data, int32_t index, int32_t color);

/**
 * Locate a layer by name; `*index` receives its iteration index.
 */
int32_t dwg_layer_index_by_name(const struct DwgData *data, const int8_t *name, int32_t *index);

/**
 * Number of entries in a symbol table (`DwgTableKind`).
 */
int32_t dwg_table_entry_count(const struct DwgData *data, int32_t table, int32_t *count);

/**
 * Copy the name of table entry `index` in the given `DwgTableKind`.
 */
int32_t dwg_table_entry_get_name(const struct DwgData *data,
                                 int32_t table,
                                 int32_t index,
                                 int8_t *buf,
                                 size_t cap,
                                 size_t *needed);

/**
 * Get the handle of the named-objects root dictionary.
 */
int32_t dwg_data_named_objects_dictionary(const struct DwgData *data, uint64_t *dictionary_handle);

/**
 * Get the number of entries in a dictionary.
 */
int32_t dwg_dictionary_count(const struct DwgData *data, uint64_t dictionary_handle, size_t *count);

/**
 * Look up an entry in a dictionary by case-insensitive key.
 */
int32_t dwg_dictionary_get(const struct DwgData *data,
                           uint64_t dictionary_handle,
                           const int8_t *key,
                           uint64_t *value);

/**
 * Read a dictionary entry by index into caller-owned output storage.
 */
int32_t dwg_dictionary_entry(const struct DwgData *data,
                             uint64_t dictionary_handle,
                             size_t index,
                             int8_t *key,
                             size_t key_capacity,
                             size_t *needed,
                             uint64_t *value);

/**
 * Set a dictionary entry, replacing a case-insensitive key when it exists.
 */
int32_t dwg_dictionary_set(struct DwgData *data,
                           uint64_t dictionary_handle,
                           const int8_t *key,
                           uint64_t value);

/**
 * Remove one dictionary entry by case-insensitive key.
 */
int32_t dwg_dictionary_remove(struct DwgData *data,
                              uint64_t dictionary_handle,
                              const int8_t *key,
                              uint64_t *value);

/**
 * Return the number of entries in an XRECORD.
 */
int32_t dwg_xrecord_count(const struct DwgData *data, uint64_t xrecord_handle, size_t *count);

/**
 * Find the physical index of a zero-based occurrence of an XRECORD group code.
 */
int32_t dwg_xrecord_find(const struct DwgData *data,
                         uint64_t xrecord_handle,
                         int32_t code,
                         size_t occurrence,
                         size_t *index);

/**
 * Read one XRECORD entry into caller-owned output storage.
 */
int32_t dwg_xrecord_entry(const struct DwgData *data,
                          uint64_t xrecord_handle,
                          size_t index,
                          int32_t *code,
                          struct DwgXrecordValue *value);

/**
 * Append one typed entry to an XRECORD.
 */
int32_t dwg_xrecord_append(struct DwgData *data,
                           uint64_t xrecord_handle,
                           int32_t code,
                           const struct DwgXrecordValue *value);

/**
 * Replace one XRECORD entry at a zero-based index.
 */
int32_t dwg_xrecord_set(struct DwgData *data,
                        uint64_t xrecord_handle,
                        size_t index,
                        int32_t code,
                        const struct DwgXrecordValue *value);

/**
 * Remove one XRECORD entry and return it through caller-owned storage.
 */
int32_t dwg_xrecord_remove(struct DwgData *data,
                           uint64_t xrecord_handle,
                           size_t index,
                           int32_t *code,
                           struct DwgXrecordValue *value);

/**
 * Return the number of XData records attached to an entity.
 */
int32_t dwg_entity_xdata_record_count(const struct DwgData *data,
                                      uint64_t entity_handle,
                                      size_t *count);

/**
 * Find a zero-based occurrence of an XData application record.
 */
int32_t dwg_entity_xdata_record_find(const struct DwgData *data,
                                     uint64_t entity_handle,
                                     const char *app_name,
                                     size_t occurrence,
                                     size_t *index);

/**
 * Read an XData record name and value count.
 */
int32_t dwg_entity_xdata_record_entry(const struct DwgData *data,
                                      uint64_t entity_handle,
                                      size_t record_index,
                                      char *application_name,
                                      size_t capacity,
                                      size_t *required,
                                      size_t *value_count);

/**
 * Append an empty XData record, registering its APPID when necessary.
 */
int32_t dwg_entity_xdata_record_append(struct DwgData *data,
                                       uint64_t entity_handle,
                                       const char *app_name,
                                       size_t *record_index);

/**
 * Remove an XData record and return its application name.
 */
int32_t dwg_entity_xdata_record_remove(struct DwgData *data,
                                       uint64_t entity_handle,
                                       size_t record_index,
                                       char *application_name,
                                       size_t capacity,
                                       size_t *required);

/**
 * Read one XData value into caller-owned storage.
 */
int32_t dwg_entity_xdata_value(const struct DwgData *data,
                               uint64_t entity_handle,
                               size_t record_index,
                               size_t value_index,
                               struct DwgXdataValue *value);

/**
 * Append one XData value.
 */
int32_t dwg_entity_xdata_value_append(struct DwgData *data,
                                      uint64_t entity_handle,
                                      size_t record_index,
                                      const struct DwgXdataValue *value);

/**
 * Replace one XData value.
 */
int32_t dwg_entity_xdata_value_set(struct DwgData *data,
                                   uint64_t entity_handle,
                                   size_t record_index,
                                   size_t value_index,
                                   const struct DwgXdataValue *value);

/**
 * Remove one XData value and return it through caller-owned storage.
 */
int32_t dwg_entity_xdata_value_remove(struct DwgData *data,
                                      uint64_t entity_handle,
                                      size_t record_index,
                                      size_t value_index,
                                      struct DwgXdataValue *value);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* ACADRUST_DWG_API_H */
