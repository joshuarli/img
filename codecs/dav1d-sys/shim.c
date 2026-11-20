#include <dav1d/dav1d.h>
#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const uint8_t *data[3];
    ptrdiff_t stride[2];
    int width, height, layout, bitdepth;
    int has_hdr, has_sequence, still, primaries, transfer, matrix, full_range;
} ImgAv1View;

typedef struct { Dav1dPicture picture; } ImgAv1Picture;

void img_av1_picture_free(ImgAv1Picture *picture) {
    if (picture) { dav1d_picture_unref(&picture->picture); free(picture); }
}

int img_av1_decode(const uint8_t *bytes, size_t len, unsigned limit,
                   ImgAv1Picture **result, ImgAv1View *view) {
    Dav1dSettings settings;
    dav1d_default_settings(&settings);
    settings.n_threads = 1;
    settings.max_frame_delay = 1;
    settings.apply_grain = 1;
    settings.frame_size_limit = limit;
    Dav1dContext *ctx = NULL;
    Dav1dData input = {0};
    ImgAv1Picture *owned = NULL;
    int status = -1, count = 0;
    *result = NULL;
    if (dav1d_open(&ctx, &settings) < 0) goto done;
    owned = calloc(1, sizeof(*owned));
    if (!owned) { status = -2; goto done; }
    uint8_t *buffer = dav1d_data_create(&input, len);
    if (!buffer) { status = -2; goto done; }
    memcpy(buffer, bytes, len);
    for (;;) {
        int rc = dav1d_send_data(ctx, &input);
        if (!rc) break;
        if (rc != DAV1D_ERR(EAGAIN)) goto done;
        Dav1dPicture pic = {0};
        if (dav1d_get_picture(ctx, &pic) < 0) goto done;
        if (++count > 1) { dav1d_picture_unref(&pic); status = -3; goto done; }
        owned->picture = pic;
    }
    for (;;) {
        Dav1dPicture pic = {0};
        int rc = dav1d_get_picture(ctx, &pic);
        if (rc < 0) { dav1d_picture_unref(&pic); break; }
        if (++count > 1) { dav1d_picture_unref(&pic); status = -3; goto done; }
        owned->picture = pic;
    }
    if (count != 1) { status = -3; goto done; }
    Dav1dPicture *p = &owned->picture;
    if (p->p.bpc == 8 && p->p.w > 0 && p->p.h > 0 && p->p.layout >= DAV1D_PIXEL_LAYOUT_I420 && p->p.layout <= DAV1D_PIXEL_LAYOUT_I444) {
        int cw = p->p.layout == DAV1D_PIXEL_LAYOUT_I444 ? p->p.w : (p->p.w + 1) / 2;
        int ch = p->p.layout == DAV1D_PIXEL_LAYOUT_I420 ? (p->p.h + 1) / 2 : p->p.h;
        for (int plane = 0; plane < 3; plane++) {
            int width = plane ? cw : p->p.w, rows = plane ? ch : p->p.h;
            ptrdiff_t stride = p->stride[!!plane];
            if (p->data[plane] && stride >= width) for (int y = 0; y < rows - 1; y++)
                memset((uint8_t *)p->data[plane] + y * stride + width, 0, (size_t)(stride - width));
        }
    }
    for (int i = 0; i < 3; i++) view->data[i] = p->data[i];
    view->stride[0] = p->stride[0]; view->stride[1] = p->stride[1];
    view->width = p->p.w; view->height = p->p.h; view->layout = p->p.layout; view->bitdepth = p->p.bpc;
    view->has_hdr = p->content_light || p->mastering_display;
    view->has_sequence = p->seq_hdr != NULL;
    if (p->seq_hdr) {
        view->still = p->seq_hdr->still_picture; view->primaries = p->seq_hdr->pri;
        view->transfer = p->seq_hdr->trc; view->matrix = p->seq_hdr->mtrx; view->full_range = p->seq_hdr->color_range;
    }
    *result = owned; owned = NULL; status = 0;
 done:
    dav1d_data_unref(&input);
    if (ctx) dav1d_close(&ctx);
    img_av1_picture_free(owned);
    return status;
}
