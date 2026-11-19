#include <png.h>
#include <spng.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

struct input { const unsigned char *data; size_t len, pos; };
static void read_png(png_structp p, png_bytep out, png_size_t n) {
    struct input *in = png_get_io_ptr(p);
    if (n > in->len - in->pos) png_error(p, "truncated");
    memcpy(out, in->data + in->pos, n); in->pos += n;
}
int native_png(const unsigned char *data, size_t len, unsigned char **out, size_t *out_len) {
    png_structp p = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
    png_infop info = p ? png_create_info_struct(p) : NULL;
    unsigned char * volatile pixels = NULL;
    png_bytep * volatile rows = NULL;
    if (!p || !info) { if (p) png_destroy_read_struct(&p, NULL, NULL); return 1; }
    if (setjmp(png_jmpbuf(p))) { free(rows); free(pixels); png_destroy_read_struct(&p, &info, NULL); return 1; }
    struct input in = {data, len, 0};
    png_set_read_fn(p, &in, read_png);
    png_set_crc_action(p, PNG_CRC_ERROR_QUIT, PNG_CRC_ERROR_QUIT);
    png_read_info(p, info);
    png_set_expand(p);
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    png_set_swap(p);
#endif
    png_set_interlace_handling(p);
    png_read_update_info(p, info);
    size_t stride = png_get_rowbytes(p, info), height = png_get_image_height(p, info);
    if (!height || stride > SIZE_MAX / height) png_error(p, "size overflow");
    pixels = malloc(stride * height); rows = malloc(height * sizeof(*rows));
    if (!pixels || !rows) png_error(p, "allocation");
    for (size_t i = 0; i < height; i++) rows[i] = pixels + i * stride;
    png_read_image(p, rows); png_read_end(p, info);
    free(rows); png_destroy_read_struct(&p, &info, NULL);
    *out = pixels; *out_len = stride * height; return 0;
}
int native_spng(const unsigned char *data, size_t len, unsigned char **out, size_t *out_len) {
    spng_ctx *ctx = spng_ctx_new(0); struct spng_ihdr ihdr; struct spng_trns trns;
    unsigned char *buf = NULL, *packed = NULL; size_t n = 0;
    if (!ctx) return 1;
    if (spng_set_png_buffer(ctx, data, len) || spng_set_crc_action(ctx, SPNG_CRC_ERROR, SPNG_CRC_ERROR) || spng_get_ihdr(ctx, &ihdr)) goto fail;
    int transparent = spng_get_trns(ctx, &trns) == 0;
    int fmt = SPNG_FMT_PNG, channels = 0, source_channels = 0;
    switch (ihdr.color_type) {
    case 0: channels = transparent ? 2 : 1; fmt = ihdr.bit_depth == 16 ? SPNG_FMT_GA16 : (transparent ? SPNG_FMT_GA8 : SPNG_FMT_G8); source_channels = ihdr.bit_depth == 16 ? 2 : channels; break;
    case 2: channels = transparent ? 4 : 3; fmt = ihdr.bit_depth == 16 ? (transparent ? SPNG_FMT_RGBA16 : SPNG_FMT_PNG) : (transparent ? SPNG_FMT_RGBA8 : SPNG_FMT_RGB8); source_channels = channels; break;
    case 3: channels = transparent ? 4 : 3; fmt = transparent ? SPNG_FMT_RGBA8 : SPNG_FMT_RGB8; source_channels = channels; break;
    case 4: channels = source_channels = 2; fmt = SPNG_FMT_PNG; break;
    case 6: channels = source_channels = 4; fmt = SPNG_FMT_PNG; break;
    default: goto fail;
    }
    if (spng_decoded_image_size(ctx, fmt, &n)) goto fail;
    buf = malloc(n); if (!buf) goto fail;
    if (spng_decode_image(ctx, buf, n, fmt, SPNG_DECODE_TRNS) || spng_decode_chunks(ctx)) goto fail;
    if (channels != source_channels) {
        size_t unit = ihdr.bit_depth == 16 ? 2 : 1, count = (size_t)ihdr.width * ihdr.height;
        packed = malloc(count * unit); if (!packed) goto fail;
        for (size_t i=0; i<count; i++) memcpy(packed + i*unit, buf + i*unit*source_channels, unit);
        free(buf); buf = packed; n = count * unit;
    }
    spng_ctx_free(ctx); *out = buf; *out_len = n; return 0;
fail:
    free(buf); spng_ctx_free(ctx); return 1;
}
