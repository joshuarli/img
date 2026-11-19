#include <setjmp.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include "jpeglib.h"

struct img_error { struct jpeg_error_mgr pub; jmp_buf jump; char *message; size_t message_len; };

static void img_fail(j_common_ptr cinfo) {
  struct img_error *error = (struct img_error *)cinfo->err;
  char buffer[JMSG_LENGTH_MAX];
  (*cinfo->err->format_message)(cinfo, buffer);
  if (error->message_len) { strncpy(error->message, buffer, error->message_len - 1); error->message[error->message_len - 1] = 0; }
  longjmp(error->jump, 1);
}

static void img_message(j_common_ptr cinfo, int level) { if (level < 0) img_fail(cinfo); }

int img_turbo_decode(const unsigned char *data, size_t size, int gray, unsigned char *out, size_t width, size_t height, size_t max_memory, char *message, size_t message_len) {
  struct jpeg_decompress_struct cinfo;
  struct img_error error;
  JSAMPROW rows[16];
  memset(&cinfo, 0, sizeof cinfo);
  cinfo.err = jpeg_std_error(&error.pub);
  error.pub.error_exit = img_fail;
  error.pub.emit_message = img_message;
  error.message = message;
  error.message_len = message_len;
  if (setjmp(error.jump)) { jpeg_destroy_decompress(&cinfo); return 0; }
  jpeg_create_decompress(&cinfo);
  cinfo.mem->max_memory_to_use = (long)max_memory;
  jpeg_mem_src(&cinfo, data, (unsigned long)size);
  jpeg_read_header(&cinfo, TRUE);
  cinfo.out_color_space = gray ? JCS_GRAYSCALE : JCS_RGB;
  cinfo.dct_method = JDCT_ISLOW;
  jpeg_start_decompress(&cinfo);
  if (cinfo.output_width != width || cinfo.output_height != height || cinfo.output_components != (gray ? 1 : 3)) {
    if (message_len) { strncpy(message, "JPEG frame differs from its marker stream", message_len - 1); message[message_len - 1] = 0; }
    jpeg_destroy_decompress(&cinfo);
    return 0;
  }
  size_t stride = width * (gray ? 1 : 3);
  while (cinfo.output_scanline < cinfo.output_height) {
    JDIMENSION count = cinfo.output_height - cinfo.output_scanline;
    if (count > 16) count = 16;
    for (JDIMENSION i = 0; i < count; i++) rows[i] = out + (size_t)(cinfo.output_scanline + i) * stride;
    jpeg_read_scanlines(&cinfo, rows, count);
  }
  jpeg_finish_decompress(&cinfo);
  jpeg_destroy_decompress(&cinfo);
  return 1;
}
