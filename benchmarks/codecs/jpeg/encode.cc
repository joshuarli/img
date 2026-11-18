#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <sys/resource.h>
#include <vector>
#ifdef USE_JPEGLI
#include "lib/jpegli/encode.h"
#define jpeg_std_error jpegli_std_error
#undef jpeg_create_compress
#define jpeg_create_compress jpegli_create_compress
#define jpeg_mem_dest jpegli_mem_dest
#define jpeg_set_defaults jpegli_set_defaults
#define jpeg_start_compress jpegli_start_compress
#define jpeg_write_scanlines jpegli_write_scanlines
#define jpeg_finish_compress jpegli_finish_compress
#define jpeg_destroy_compress jpegli_destroy_compress
#else
#include "jpeglib.h"
#endif

static double millis(const timespec& t) { return t.tv_sec * 1000.0 + t.tv_nsec / 1000000.0; }
static double user(const rusage& r) { return r.ru_utime.tv_sec * 1000.0 + r.ru_utime.tv_usec / 1000.0; }
int main(int argc, char** argv) {
    if (argc < 4) return 2;
    FILE* input = fopen(argv[2], "rb"); if (!input) return 3;
    char magic[3]; unsigned w, h, max;
    if (fscanf(input, "%2s %u %u %u", magic, &w, &h, &max) != 4 || fgetc(input) != '\n' || max != 255) return 4;
    int channels = !strcmp(magic,"P5") ? 1 : !strcmp(magic,"P6") ? 3 : 0;
    if (!channels) return 4;
    std::vector<unsigned char> pixels((size_t)w*h*channels);
    if (fread(pixels.data(), 1, pixels.size(), input) != pixels.size()) return 4;
    fclose(input);
    int quality = atoi(argv[3]);
    timespec start, end; rusage before, after;
    getrusage(RUSAGE_SELF, &before); clock_gettime(CLOCK_MONOTONIC, &start);
    jpeg_compress_struct c{}; jpeg_error_mgr e{}; c.err=jpeg_std_error(&e);
    jpeg_create_compress(&c); unsigned char* out=nullptr; unsigned long len=0; jpeg_mem_dest(&c,&out,&len);
    c.image_width=w; c.image_height=h; c.input_components=channels; c.in_color_space=channels==1?JCS_GRAYSCALE:JCS_RGB;
    jpeg_set_defaults(&c);
    c.comp_info[0].h_samp_factor = strstr(argv[1],"420") ? 2 : 1;
    c.comp_info[0].v_samp_factor = strstr(argv[1],"420") ? 2 : 1;
    if (channels == 3) {
        for (int i=1; i<3; i++) c.comp_info[i].h_samp_factor = c.comp_info[i].v_samp_factor = 1;
    }
#ifdef USE_JPEGLI
    jpegli_set_distance(&c, jpegli_quality_to_distance(quality), TRUE);
    jpegli_set_progressive_level(&c, 2);
    c.optimize_coding = TRUE;
#else
    jpeg_set_quality(&c, quality, TRUE);
#endif
    jpeg_start_compress(&c, TRUE);
    while (c.next_scanline < c.image_height) {
        JSAMPROW row = pixels.data() + (size_t)c.next_scanline * w * channels;
        jpeg_write_scanlines(&c, &row, 1);
    }
    jpeg_finish_compress(&c); jpeg_destroy_compress(&c);
    clock_gettime(CLOCK_MONOTONIC, &end); getrusage(RUSAGE_SELF, &after);
    if (argc > 4) { FILE* f=fopen(argv[4], "wb"); if (!f || fwrite(out,1,len,f)!=len || fclose(f)) return 5; }
    printf("{\"wall_ms\":%.6f,\"cpu_ms\":%.6f,\"rss_bytes\":%ld,\"output_bytes\":%lu}\n", millis(end)-millis(start),user(after)-user(before),after.ru_maxrss,len);
    free(out); return 0;
}
