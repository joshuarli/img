#include <dav1d/dav1d.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc != 4) return 1;
    FILE *f = fopen(argv[1], "rb"); if (!f) return 2;
    if (fseek(f, 0, SEEK_END)) return 3; long n = ftell(f); if (n < 0) return 3; rewind(f);
    Dav1dSettings s; dav1d_default_settings(&s); s.n_threads = 1; s.max_frame_delay = 1; s.apply_grain = atoi(argv[3]);
    Dav1dContext *ctx = NULL; Dav1dData data = {0}; Dav1dPicture p = {0};
    if (dav1d_open(&ctx, &s) < 0) return 4;
    uint8_t *buf = dav1d_data_create(&data, (size_t)n); if (!buf || fread(buf, 1, n, f) != (size_t)n) return 5; fclose(f);
    if (dav1d_send_data(ctx, &data) < 0 || dav1d_get_picture(ctx, &p) < 0 || p.p.bpc != 8 || !p.seq_hdr) return 6;
    int w = p.p.w, h = p.p.h, layout = p.p.layout, cw = layout == 3 ? w : (w + 1) / 2, ch = layout == 1 ? (h + 1) / 2 : h;
    if (layout < 1 || layout > 3) return 7;
    f = fopen(argv[2], "wb"); if (!f) return 8;
    uint32_t header[] = { (uint32_t)w, (uint32_t)h, (uint32_t)layout, p.seq_hdr->color_range };
    if (fwrite(header, sizeof(header), 1, f) != 1) return 9;
    for (int plane = 0; plane < 3; plane++) for (int y = 0; y < (plane ? ch : h); y++) {
        size_t width = plane ? cw : w;
        if (fwrite((uint8_t *)p.data[plane] + y * p.stride[!!plane], 1, width, f) != width) return 10;
    }
    fclose(f); dav1d_picture_unref(&p); dav1d_data_unref(&data); dav1d_close(&ctx); return 0;
}
