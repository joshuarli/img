#include <cstdio>
#include <cstdlib>
#include <vector>
#include "lib/jpegli/decode.h"
extern "C" long img_jpegli_decode(const unsigned char* data, size_t size, unsigned char** result) {
  jpeg_decompress_struct d; jpeg_error_mgr e;
  d.err = jpegli_std_error(&e);
  jpegli_create_decompress(&d);
  jpegli_mem_src(&d, data, size);
  jpegli_read_header(&d, TRUE);
  d.out_color_space = d.num_components == 1 ? JCS_GRAYSCALE : JCS_RGB;
  jpegli_start_decompress(&d);
  size_t stride = size_t(d.output_width) * d.output_components;
  size_t need = stride * d.output_height;
  unsigned char* out = static_cast<unsigned char*>(malloc(need)); *result = out;
  while (d.output_scanline < d.output_height) { JSAMPROW row = out + d.output_scanline * stride; jpegli_read_scanlines(&d, &row, 1); }
  jpegli_finish_decompress(&d); jpegli_destroy_decompress(&d);
  return long(need);
}
