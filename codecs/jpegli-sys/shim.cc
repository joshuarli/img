#include <cstdlib>
#include <cstring>
#include <climits>
#include "lib/jpegli/encode.h"

struct Encoder {
  jpeg_compress_struct jpeg;
  jpeg_error_mgr error;
  jpeg_destination_mgr dest;
  unsigned char* output;
  size_t capacity, size, limit;
};
struct JpegliError {};
// libjpeg requires error_exit not to return; throwing unwinds jpegli's C++ frames to the API boundary.
static void fail(j_common_ptr) { throw JpegliError{}; }
static void init(j_compress_ptr) {}
static boolean grow(j_compress_ptr c) {
  auto* e = reinterpret_cast<Encoder*>(c);
  if (e->capacity >= e->limit) fail(reinterpret_cast<j_common_ptr>(c));
  size_t next = e->capacity > e->limit / 2 ? e->limit : e->capacity * 2;
  auto* p = static_cast<unsigned char*>(realloc(e->output, next));
  if (!p) fail(reinterpret_cast<j_common_ptr>(c));
  e->output = p;
  e->dest.next_output_byte = p + e->capacity;
  e->dest.free_in_buffer = next - e->capacity;
  e->capacity = next;
  return TRUE;
}
static void term(j_compress_ptr c) {
  auto* e = reinterpret_cast<Encoder*>(c);
  e->size = e->capacity - e->dest.free_in_buffer;
}
extern "C" Encoder* img_jpegli_create(unsigned width, unsigned height, int gray, float quality, int trellis, size_t memory, size_t limit) {
  auto* e = static_cast<Encoder*>(calloc(1, sizeof(Encoder)));
  if (!e) return nullptr;
  e->jpeg.err = jpegli_std_error(&e->error); e->error.error_exit = fail;
  try {
    jpegli_create_compress(&e->jpeg);
    e->jpeg.mem->max_memory_to_use = memory > LONG_MAX ? LONG_MAX : static_cast<long>(memory);
    e->limit = limit; e->capacity = limit < 65536 ? limit : 65536;
    if (!e->capacity || !(e->output = static_cast<unsigned char*>(malloc(e->capacity)))) fail(reinterpret_cast<j_common_ptr>(&e->jpeg));
    e->dest.init_destination = init; e->dest.empty_output_buffer = grow; e->dest.term_destination = term;
    e->dest.next_output_byte = e->output; e->dest.free_in_buffer = e->capacity; e->jpeg.dest = &e->dest;
    e->jpeg.image_width = width; e->jpeg.image_height = height;
    e->jpeg.input_components = gray ? 1 : 3; e->jpeg.in_color_space = gray ? JCS_GRAYSCALE : JCS_RGB;
    jpegli_set_defaults(&e->jpeg);
    if (!gray) { e->jpeg.comp_info[0].h_samp_factor = 1; e->jpeg.comp_info[0].v_samp_factor = 1; }
    float distance = quality >= 100 ? 0.01f : quality >= 30 ? 0.1f + (100 - quality) * 0.09f : 53.0f / 3000.0f * quality * quality - 23.0f / 20.0f * quality + 25.0f;
    jpegli_set_distance(&e->jpeg, distance, TRUE);
    jpegli_enable_adaptive_quantization(&e->jpeg, distance < 0.3f);
    jpegli_enable_trellis_quantization(&e->jpeg, trellis);
    jpegli_set_progressive_level(&e->jpeg, 2); e->jpeg.optimize_coding = TRUE;
    jpegli_start_compress(&e->jpeg, TRUE);
    return e;
  } catch (...) {
    if (e->jpeg.mem) jpegli_destroy_compress(&e->jpeg);
    free(e->output); free(e); return nullptr;
  }
}
extern "C" int img_jpegli_marker(Encoder* e, int marker, const unsigned char* data, size_t len) {
  try {
    if (marker == 2) jpegli_write_icc_profile(&e->jpeg, data, len);
    else jpegli_write_marker(&e->jpeg, JPEG_APP0 + marker, data, len);
    return 1;
  } catch (...) { return 0; }
}
extern "C" int img_jpegli_rows(Encoder* e, const unsigned char* data, size_t stride, unsigned count) {
  try {
    for (unsigned i = 0; i < count; ++i) { JSAMPROW row = const_cast<JSAMPROW>(data + i * stride); if (jpegli_write_scanlines(&e->jpeg, &row, 1) != 1) return 0; }
    return 1;
  } catch (...) { return 0; }
}
extern "C" int img_jpegli_finish(Encoder* e, const unsigned char** data, size_t* size) {
  try {
    jpegli_finish_compress(&e->jpeg); *data = e->output; *size = e->size; return 1;
  } catch (...) { return 0; }
}
extern "C" void img_jpegli_destroy(Encoder* e) { jpegli_destroy_compress(&e->jpeg); free(e->output); free(e); }
