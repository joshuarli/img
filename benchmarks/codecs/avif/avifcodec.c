#include <avif/avif.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

static double now_ms(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec * 1000.0 + t.tv_nsec / 1e6;
}

static double cpu_ms(void) {
    struct rusage r;
    getrusage(RUSAGE_SELF, &r);
    return (r.ru_utime.tv_sec + r.ru_stime.tv_sec) * 1000.0 +
           (r.ru_utime.tv_usec + r.ru_stime.tv_usec) / 1000.0;
}

static long peak_rss(void) {
    struct rusage r;
    getrusage(RUSAGE_SELF, &r);
    return r.ru_maxrss;
}

static void report(double start, double cpu_before, size_t output_bytes) {
    printf("{\"wall_ms\":%.3f,\"cpu_ms\":%.3f,\"rss_bytes\":%ld,\"output_bytes\":%zu}\n",
           now_ms() - start, cpu_ms() - cpu_before, peak_rss(), output_bytes);
}

static int read_pnm(const char *path, uint32_t *width, uint32_t *height, uint8_t **pixels)
{
    FILE *f = fopen(path, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", path); return 1; }
    char magic[3] = {0};
    if (fscanf(f, "%2s", magic) != 1 || strcmp(magic, "P6") != 0) {
        fprintf(stderr, "expected P6 PNM\n"); fclose(f); return 1;
    }
    int c;
    unsigned values[3];
    for (int i = 0; i < 3; i++) {
        for (;;) {
            c = fgetc(f);
            if (c == '#') { while ((c = fgetc(f)) != EOF && c != '\n') {} continue; }
            if (c != EOF && (c == ' ' || c == '\t' || c == '\r' || c == '\n')) continue;
            break;
        }
        if (c == EOF) { fclose(f); return 1; }
        ungetc(c, f);
        if (fscanf(f, "%u", &values[i]) != 1) { fclose(f); return 1; }
    }
    fgetc(f);
    *width = values[0];
    *height = values[1];
    if (values[2] != 255 || !*width || !*height) {
        fprintf(stderr, "expected 8-bit PNM\n"); fclose(f); return 1;
    }
    size_t size = (size_t)*width * *height * 3;
    *pixels = malloc(size);
    if (!*pixels || fread(*pixels, 1, size, f) != size) {
        fprintf(stderr, "truncated PNM\n"); fclose(f); return 1;
    }
    fclose(f);
    return 0;
}

static int write_file(const char *path, const void *data, size_t size)
{
    FILE *f = fopen(path, "wb");
    if (!f) { fprintf(stderr, "cannot write %s\n", path); return 1; }
    if (fwrite(data, 1, size, f) != size) { fclose(f); return 1; }
    fclose(f);
    return 0;
}

static int encode(const char *input, const char *output, uint32_t width, uint32_t height,
                  int chroma, int speed, int quality, int threads, const char *codec)
{
    uint32_t pnm_width, pnm_height;
    uint8_t *pixels = NULL;
    if (read_pnm(input, &pnm_width, &pnm_height, &pixels)) return 1;
    if (pnm_width != width || pnm_height != height) {
        fprintf(stderr, "dimensions mismatch\n"); free(pixels); return 1;
    }
    avifImage *image = avifImageCreate(width, height, 8,
        chroma == 420 ? AVIF_PIXEL_FORMAT_YUV420 : AVIF_PIXEL_FORMAT_YUV444);
    if (!image) return 1;
    image->yuvRange = AVIF_RANGE_FULL;
    image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
    image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT601;

    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, image);
    rgb.depth = 8;
    rgb.format = AVIF_RGB_FORMAT_RGB;
    rgb.avoidLibYUV = AVIF_TRUE;
    rgb.pixels = pixels;
    rgb.rowBytes = width * 3;

    avifEncoder *encoder = avifEncoderCreate();
    if (!encoder) return 1;
    encoder->codecChoice = strcmp(codec, "svt") == 0 ? AVIF_CODEC_CHOICE_SVT : AVIF_CODEC_CHOICE_AOM;
    encoder->speed = speed;
    encoder->quality = quality;
    encoder->maxThreads = threads;
    avifRWData data = AVIF_DATA_EMPTY;

    double cpu_before = cpu_ms();
    double start = now_ms();
    avifResult result = avifImageRGBToYUV(image, &rgb);
    if (result == AVIF_RESULT_OK) {
        result = avifEncoderAddImage(encoder, image, 1, AVIF_ADD_IMAGE_FLAG_SINGLE);
    }
    if (result == AVIF_RESULT_OK) result = avifEncoderFinish(encoder, &data);
    if (result == AVIF_RESULT_OK) {
        /* img 4485043 reads nclx full_range_flag from bit 0 instead of bit 7 and would
           misdecode full-range output. Renaming the optional colr property to free leaves
           the AV1 sequence header CICP authoritative, as the current ravif output does. */
        for (size_t i = 0; i + 8 <= data.size; i++) {
            if (memcmp(data.data + i, "colrnclx", 8) == 0) {
                memcpy(data.data + i, "free", 4);
                break;
            }
        }
    }
    double elapsed = now_ms() - start;
    double cpu_after = cpu_ms() - cpu_before;

    int status = 1;
    if (result != AVIF_RESULT_OK) {
        fprintf(stderr, "encode failed: %s\n", avifResultToString(result));
    } else if (write_file(output, data.data, data.size)) {
        status = 1;
    } else {
        printf("{\"wall_ms\":%.3f,\"cpu_ms\":%.3f,\"rss_bytes\":%ld,\"output_bytes\":%zu}\n",
               elapsed, cpu_after, peak_rss(), data.size);
        status = 0;
    }
    avifRWDataFree(&data);
    avifEncoderDestroy(encoder);
    avifImageDestroy(image);
    free(pixels);
    return status;
}

static int decode(const char *input, const char *output)
{
    FILE *f = fopen(input, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", input); return 1; }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    uint8_t *bytes = malloc((size_t)size);
    if (!bytes || fread(bytes, 1, (size_t)size, f) != (size_t)size) { fclose(f); return 1; }
    fclose(f);

    avifDecoder *decoder = avifDecoderCreate();
    if (!decoder) return 1;
    decoder->maxThreads = 1;
    decoder->codecChoice = AVIF_CODEC_CHOICE_DAV1D;

    double cpu_before = cpu_ms();
    double start = now_ms();
    avifResult result = avifDecoderSetIOMemory(decoder, bytes, (size_t)size);
    if (result == AVIF_RESULT_OK) result = avifDecoderParse(decoder);
    if (result == AVIF_RESULT_OK) result = avifDecoderNextImage(decoder);
    avifRGBImage rgb;
    memset(&rgb, 0, sizeof(rgb));
    uint8_t *pixels = NULL;
    size_t pixel_bytes = 0;
    uint32_t width = 0, height = 0;
    if (result == AVIF_RESULT_OK) {
        avifRGBImageSetDefaults(&rgb, decoder->image);
        rgb.depth = 8;
        rgb.format = AVIF_RGB_FORMAT_RGB;
        rgb.avoidLibYUV = AVIF_TRUE;
        result = avifRGBImageAllocatePixels(&rgb);
        if (result == AVIF_RESULT_OK) result = avifImageYUVToRGB(decoder->image, &rgb);
    }
    if (result == AVIF_RESULT_OK) {
        width = rgb.width;
        height = rgb.height;
        pixel_bytes = (size_t)width * height * 3;
        pixels = malloc(pixel_bytes);
        if (!pixels) result = AVIF_RESULT_OUT_OF_MEMORY;
    }
    if (result == AVIF_RESULT_OK) {
        for (uint32_t y = 0; y < height; y++) {
            memcpy(pixels + (size_t)y * width * 3, rgb.pixels + (size_t)y * rgb.rowBytes, (size_t)width * 3);
        }
        if (output) result = write_file(output, pixels, pixel_bytes) ? AVIF_RESULT_UNKNOWN_ERROR : AVIF_RESULT_OK;
    }
    double elapsed = now_ms() - start;
    double cpu_after = cpu_ms() - cpu_before;

    int status = 1;
    if (result != AVIF_RESULT_OK) {
        fprintf(stderr, "decode failed: %s\n", avifResultToString(result));
    } else {
        printf("{\"wall_ms\":%.3f,\"cpu_ms\":%.3f,\"rss_bytes\":%ld,\"output_bytes\":%zu}\n",
               elapsed, cpu_after, peak_rss(), pixel_bytes);
        status = 0;
    }
    free(pixels);
    avifRGBImageFreePixels(&rgb);
    avifDecoderDestroy(decoder);
    free(bytes);
    return status;
}

int main(int argc, char **argv)
{
    if (argc >= 2 && strcmp(argv[1], "encode") == 0 && (argc == 10 || argc == 11)) {
        /* encode INPUT OUTPUT WIDTH HEIGHT CHROMA SPEED QUALITY THREADS [CODEC] */
        return encode(argv[2], argv[3], (uint32_t)strtoul(argv[4], NULL, 10),
                      (uint32_t)strtoul(argv[5], NULL, 10), atoi(argv[6]),
                      atoi(argv[7]), atoi(argv[8]), atoi(argv[9]),
                      argc == 11 ? argv[10] : "aom");
    }
    if (argc == 3 && strcmp(argv[1], "decode") == 0) {
        return decode(argv[2], NULL);
    }
    if (argc == 4 && strcmp(argv[1], "decode") == 0) {
        return decode(argv[2], argv[3]);
    }
    fprintf(stderr, "usage:\n  avifcodec encode IMG A B C D E F G\n  avifcodec decode FILE [OUT.rgb]\n");
    return 2;
}
