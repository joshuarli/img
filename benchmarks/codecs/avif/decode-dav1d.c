#include <avif/avif.h>
#include <dav1d/dav1d.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct { uint32_t w, h, chroma, matrix, full; uint8_t *pixels; } Planes;

static int copy_planes(Planes *out, uint32_t w, uint32_t h, unsigned layout, unsigned matrix, unsigned full, const uint8_t *const data[3], const size_t stride[3]) {
    if (layout != 1 && layout != 2 && layout != 3) return -1;
    size_t cw = layout == 3 ? w : (w + 1) / 2;
    size_t ch = layout == 1 ? (h + 1) / 2 : h;
    size_t sizes[3] = { (size_t)w * h, cw * ch, cw * ch };
    size_t widths[3] = { w, cw, cw }, heights[3] = { h, ch, ch };
    out->pixels = malloc(sizes[0] + sizes[1] + sizes[2]);
    if (!out->pixels) return -1;
    uint8_t *dst = out->pixels;
    for (int p = 0; p < 3; p++) {
        for (size_t y = 0; y < heights[p]; y++) memcpy(dst + y * widths[p], data[p] + y * stride[p], widths[p]);
        dst += sizes[p];
    }
    out->w = w; out->h = h; out->chroma = layout; out->matrix = matrix; out->full = full;
    return 0;
}

int decode_raw(const uint8_t *bytes, size_t len, Planes *out) {
    Dav1dSettings s; dav1d_default_settings(&s); s.n_threads = 1; s.max_frame_delay = 1; s.apply_grain = 1;
    Dav1dContext *ctx = NULL; Dav1dData data = {0}; Dav1dPicture pic = {0};
    if (dav1d_open(&ctx, &s) < 0) return -1;
    uint8_t *buf = dav1d_data_create(&data, len);
    int rc = -1;
    if (!buf) goto done;
    memcpy(buf, bytes, len);
    if (dav1d_send_data(ctx, &data) < 0 || dav1d_get_picture(ctx, &pic) < 0 || pic.p.bpc != 8 || !pic.seq_hdr) goto done;
    const uint8_t *planes[3] = {pic.data[0], pic.data[1], pic.data[2]};
    size_t strides[3] = {(size_t)pic.stride[0], (size_t)pic.stride[1], (size_t)pic.stride[1]};
    rc = copy_planes(out, pic.p.w, pic.p.h, pic.p.layout, pic.seq_hdr->mtrx, pic.seq_hdr->color_range, planes, strides);
 done:
    dav1d_picture_unref(&pic); dav1d_data_unref(&data); dav1d_close(&ctx); return rc;
}

int decode_avif(const uint8_t *bytes, size_t len, Planes *out) {
    avifDecoder *d = avifDecoderCreate();
    if (!d) return -1;
    d->maxThreads = 1; d->codecChoice = AVIF_CODEC_CHOICE_DAV1D;
    int rc = -1;
    if (avifDecoderSetIOMemory(d, bytes, len) != AVIF_RESULT_OK || avifDecoderParse(d) != AVIF_RESULT_OK || avifDecoderNextImage(d) != AVIF_RESULT_OK || d->image->depth != 8) goto done;
    avifImage *i = d->image;
    unsigned layout = i->yuvFormat == AVIF_PIXEL_FORMAT_YUV420 ? 1 : i->yuvFormat == AVIF_PIXEL_FORMAT_YUV422 ? 2 : i->yuvFormat == AVIF_PIXEL_FORMAT_YUV444 ? 3 : 0;
    const uint8_t *planes[3] = {i->yuvPlanes[0], i->yuvPlanes[1], i->yuvPlanes[2]};
    size_t strides[3] = {i->yuvRowBytes[0], i->yuvRowBytes[1], i->yuvRowBytes[2]};
    rc = copy_planes(out, i->width, i->height, layout, i->matrixCoefficients, i->yuvRange, planes, strides);
 done:
    avifDecoderDestroy(d); return rc;
}
