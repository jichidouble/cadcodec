/* Collection-setter smoke test for the public C ABI.
 *
 * Usage: c_entity_set_collections <fixture.dwg>
 */
#include <stddef.h>
#include <stdint.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;

enum {
  DWG_OK = 0,
  DWG_EINVAL = 1,
  DWG_OBJECT_SPLINE = 8,
  DWG_OBJECT_LWPOLYLINE = 9,
  DWG_OBJECT_POLYLINE2D = 15,
  DWG_OBJECT_POLYLINE3D = 16,
  DWG_OBJECT_MESH = 17,
  DWG_OBJECT_POLYFACE_MESH = 19,
  DWG_OBJECT_LEADER = 22,
};

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);

extern int dwg_entity_spline_get_control_point_count(const DwgObject *obj,
                                                     int *count);
extern int dwg_entity_spline_set_control_point(DwgData *data,
                                                const DwgObject *obj, int index,
                                                double x, double y, double z);
extern int dwg_entity_spline_append_control_point(DwgData *data,
                                                  const DwgObject *obj,
                                                  double x, double y, double z);
extern int dwg_entity_spline_remove_control_point(DwgData *data,
                                                  const DwgObject *obj,
                                                  int index);

extern int dwg_entity_lwpolyline_get_vertex_count(const DwgObject *obj,
                                                  int *count);
extern int dwg_entity_lwpolyline_set_vertex(DwgData *data,
                                             const DwgObject *obj, int index,
                                             double x, double y);
extern int dwg_entity_lwpolyline_append_vertex(DwgData *data,
                                               const DwgObject *obj, double x,
                                               double y);
extern int dwg_entity_lwpolyline_remove_vertex(DwgData *data,
                                               const DwgObject *obj, int index);
extern int dwg_entity_lwpolyline_clear_vertices(DwgData *data,
                                                const DwgObject *obj);

extern int dwg_entity_polyline2d_set_vertex(DwgData *data,
                                             const DwgObject *obj, int index,
                                             double x, double y, double z);
extern int dwg_entity_polyline2d_append_vertex(DwgData *data,
                                               const DwgObject *obj, double x,
                                               double y, double z);
extern int dwg_entity_polyline2d_remove_vertex(DwgData *data,
                                               const DwgObject *obj, int index);
extern int dwg_entity_polyline2d_clear_vertices(DwgData *data,
                                                const DwgObject *obj);

extern int dwg_entity_polyline3d_set_vertex(DwgData *data,
                                             const DwgObject *obj, int index,
                                             double x, double y, double z);
extern int dwg_entity_polyline3d_append_vertex(DwgData *data,
                                               const DwgObject *obj, double x,
                                               double y, double z);
extern int dwg_entity_polyline3d_remove_vertex(DwgData *data,
                                               const DwgObject *obj, int index);
extern int dwg_entity_polyline3d_clear_vertices(DwgData *data,
                                                const DwgObject *obj);

extern int dwg_entity_leader_set_vertex(DwgData *data, const DwgObject *obj,
                                        int index, double x, double y,
                                        double z);
extern int dwg_entity_leader_append_vertex(DwgData *data,
                                           const DwgObject *obj, double x,
                                           double y, double z);
extern int dwg_entity_leader_remove_vertex(DwgData *data,
                                           const DwgObject *obj, int index);
extern int dwg_entity_leader_clear_vertices(DwgData *data,
                                            const DwgObject *obj);

extern int dwg_entity_mesh_get_vertex_count(const DwgObject *obj, int *count);
extern int dwg_entity_mesh_set_vertex(DwgData *data, const DwgObject *obj,
                                       int index, double x, double y,
                                       double z);
extern int dwg_entity_mesh_append_vertex(DwgData *data, const DwgObject *obj,
                                          double x, double y, double z);
extern int dwg_entity_mesh_remove_vertex(DwgData *data, const DwgObject *obj,
                                         int index);
extern int dwg_entity_mesh_get_face_count(const DwgObject *obj, int *count);
extern int dwg_entity_mesh_get_face_vertex(const DwgObject *obj, int face,
                                           int slot, uint32_t *vertex);
extern int dwg_entity_mesh_set_face_vertex(DwgData *data,
                                            const DwgObject *obj, int face,
                                            int slot, uint32_t vertex);
extern int dwg_entity_mesh_set_face(DwgData *data, const DwgObject *obj,
                                    int face, const uint32_t *indices,
                                    size_t count);
extern int dwg_entity_mesh_append_face(DwgData *data, const DwgObject *obj,
                                       const uint32_t *indices, size_t count);
extern int dwg_entity_mesh_remove_face(DwgData *data, const DwgObject *obj,
                                       int face);
extern int dwg_entity_mesh_clear_faces(DwgData *data, const DwgObject *obj);
extern int dwg_entity_mesh_clear_vertices(DwgData *data,
                                          const DwgObject *obj);

extern int dwg_entity_polyface_mesh_set_vertex(DwgData *data,
                                                const DwgObject *obj, int index,
                                                double x, double y, double z);
extern int dwg_entity_polyface_mesh_append_vertex(DwgData *data,
                                                  const DwgObject *obj,
                                                  double x, double y, double z);
extern int dwg_entity_polyface_mesh_remove_vertex(DwgData *data,
                                                  const DwgObject *obj,
                                                  int index);
extern int dwg_entity_polyface_mesh_clear_vertices(DwgData *data,
                                                   const DwgObject *obj);

static const DwgObject *find_type(const DwgData *data, int want) {
  int error = 0;
  const DwgObject *obj = dwg_data_first_object(data, &error);
  if (error != 0)
    return NULL;
  while (obj != NULL) {
    if (dwg_object_get_type(obj) == want)
      return obj;
    obj = dwg_object_next(obj);
  }
  return NULL;
}

int main(int argc, char **argv) {
  DwgData *data = NULL;
  const DwgObject *spline = NULL;
  const DwgObject *lw = NULL;
  const DwgObject *p2 = NULL;
  const DwgObject *p3 = NULL;
  const DwgObject *leader = NULL;
  const DwgObject *mesh = NULL;
  const DwgObject *polyface = NULL;
  uint32_t face[] = {0, 2, 3};
  uint32_t appended_face[] = {1, 2, 3};
  uint32_t vertex = UINT32_MAX;
  int count = 0;
  int exit_code = 0;

  if (argc < 2)
    return 1;
  if (dwg_data_create(&data) != DWG_OK)
    return 2;
  if (dwg_read_file(argv[1], data) != DWG_OK) {
    exit_code = 3;
    goto cleanup;
  }

  spline = find_type(data, DWG_OBJECT_SPLINE);
  lw = find_type(data, DWG_OBJECT_LWPOLYLINE);
  p2 = find_type(data, DWG_OBJECT_POLYLINE2D);
  p3 = find_type(data, DWG_OBJECT_POLYLINE3D);
  leader = find_type(data, DWG_OBJECT_LEADER);
  mesh = find_type(data, DWG_OBJECT_MESH);
  polyface = find_type(data, DWG_OBJECT_POLYFACE_MESH);
  if (spline == NULL || lw == NULL || p2 == NULL || p3 == NULL ||
      leader == NULL || mesh == NULL || polyface == NULL) {
    exit_code = 4;
    goto cleanup;
  }

  if (dwg_entity_spline_get_control_point_count(spline, &count) != DWG_OK ||
      count != 5 ||
      dwg_entity_spline_set_control_point(data, spline, 0, 10.0, 11.0, 12.0) != DWG_OK ||
      dwg_entity_spline_append_control_point(data, spline, 5.0, 5.0, 5.0) != DWG_OK ||
      dwg_entity_spline_get_control_point_count(spline, &count) != DWG_OK ||
      count != 6 ||
      dwg_entity_spline_remove_control_point(data, spline, 5) != DWG_OK)
    exit_code = 10;

  if (exit_code == 0 &&
      (dwg_entity_lwpolyline_get_vertex_count(lw, &count) != DWG_OK ||
       count != 2 || dwg_entity_lwpolyline_set_vertex(data, lw, 0, 2.0, 3.0) != DWG_OK ||
       dwg_entity_lwpolyline_append_vertex(data, lw, 4.0, 5.0) != DWG_OK ||
       dwg_entity_lwpolyline_remove_vertex(data, lw, 2) != DWG_OK ||
       dwg_entity_lwpolyline_clear_vertices(data, lw) != DWG_OK))
    exit_code = 11;

  if (exit_code == 0 &&
      (dwg_entity_polyline2d_set_vertex(data, p2, 0, 2.0, 3.0, 4.0) != DWG_OK ||
       dwg_entity_polyline2d_append_vertex(data, p2, 4.0, 5.0, 6.0) != DWG_OK ||
       dwg_entity_polyline2d_remove_vertex(data, p2, 2) != DWG_OK ||
       dwg_entity_polyline2d_clear_vertices(data, p2) != DWG_OK ||
       dwg_entity_polyline3d_set_vertex(data, p3, 0, 2.0, 3.0, 4.0) != DWG_OK ||
       dwg_entity_polyline3d_append_vertex(data, p3, 4.0, 5.0, 6.0) != DWG_OK ||
       dwg_entity_polyline3d_remove_vertex(data, p3, 2) != DWG_OK ||
       dwg_entity_polyline3d_clear_vertices(data, p3) != DWG_OK))
    exit_code = 12;

  if (exit_code == 0 &&
      (dwg_entity_leader_set_vertex(data, leader, 0, 2.0, 3.0, 4.0) != DWG_OK ||
       dwg_entity_leader_append_vertex(data, leader, 4.0, 5.0, 6.0) != DWG_OK ||
       dwg_entity_leader_remove_vertex(data, leader, 2) != DWG_OK ||
       dwg_entity_leader_clear_vertices(data, leader) != DWG_OK))
    exit_code = 13;

  if (exit_code == 0 &&
      (dwg_entity_mesh_get_vertex_count(mesh, &count) != DWG_OK || count != 4 ||
       dwg_entity_mesh_set_vertex(data, mesh, 0, 2.0, 3.0, 4.0) != DWG_OK ||
       dwg_entity_mesh_append_vertex(data, mesh, 4.0, 5.0, 6.0) != DWG_OK ||
       dwg_entity_mesh_remove_vertex(data, mesh, 4) != DWG_OK ||
       dwg_entity_mesh_get_face_count(mesh, &count) != DWG_OK || count != 1 ||
       dwg_entity_mesh_get_face_vertex(mesh, 0, 1, &vertex) != DWG_OK || vertex != 1 ||
       dwg_entity_mesh_set_face_vertex(data, mesh, 0, 1, 3) != DWG_OK ||
       dwg_entity_mesh_set_face(data, mesh, 0, face, 3) != DWG_OK ||
       dwg_entity_mesh_append_face(data, mesh, appended_face, 3) != DWG_OK ||
       dwg_entity_mesh_remove_face(data, mesh, 0) != DWG_OK ||
       dwg_entity_mesh_clear_faces(data, mesh) != DWG_OK ||
       dwg_entity_mesh_set_face(data, mesh, 0, face, 2) != DWG_EINVAL ||
       dwg_entity_mesh_clear_vertices(data, mesh) != DWG_OK))
    exit_code = 14;

  if (exit_code == 0 &&
      (dwg_entity_polyface_mesh_set_vertex(data, polyface, 0, 2.0, 3.0, 4.0) != DWG_OK ||
       dwg_entity_polyface_mesh_append_vertex(data, polyface, 4.0, 5.0, 6.0) != DWG_OK ||
       dwg_entity_polyface_mesh_remove_vertex(data, polyface, 2) != DWG_OK ||
       dwg_entity_polyface_mesh_clear_vertices(data, polyface) != DWG_OK))
    exit_code = 15;

cleanup:
  dwg_data_destroy(data);
  return exit_code;
}
