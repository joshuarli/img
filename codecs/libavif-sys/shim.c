#include <avif/avif.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int aomQualityToQuantizer(int quality, avifBool isTuneIq);
int av1_quantizer_to_qindex(int quantizer);

static int qindex_at(int quality) { return av1_quantizer_to_qindex(aomQualityToQuantizer(quality, AVIF_TRUE)); }

/* Integer qualities reach the qindex libavif's tune=iq mapping gives them; fractional qualities interpolate between neighbors. */
int img_avif_qindex(float quality) {
    if (!(quality > 1.0f)) return qindex_at(1);
    if (quality >= 100.0f) return 0;
    int below = (int)floorf(quality);
    float qindex = (float)qindex_at(below) + (quality - (float)below) * (float)(qindex_at(below + 1) - qindex_at(below));
    return (int)lroundf(qindex);
}

int img_avif_encode(uint32_t width, uint32_t height, const uint8_t *pixels, int speed, int qindex,
                    const uint8_t *exif, size_t exif_size, avifRWData *output) {
    avifImage *image = avifImageCreate(width, height, 8, AVIF_PIXEL_FORMAT_YUV444);
    if (!image) return AVIF_RESULT_OUT_OF_MEMORY;
    avifEncoder *encoder = avifEncoderCreate();
    if (!encoder) { avifImageDestroy(image); return AVIF_RESULT_OUT_OF_MEMORY; }
    image->yuvRange = AVIF_RANGE_FULL;
    image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
    image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT601;
    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, image);
    rgb.depth = 8;
    rgb.format = AVIF_RGB_FORMAT_RGB;
    rgb.avoidLibYUV = AVIF_TRUE;
    rgb.pixels = (uint8_t *)pixels;
    rgb.rowBytes = width * 3;
    encoder->codecChoice = AVIF_CODEC_CHOICE_AOM;
    encoder->speed = speed;
    encoder->quality = qindex == 0 ? AVIF_QUALITY_LOSSLESS : 50;
    encoder->maxThreads = 1;
    avifResult result = exif ? avifImageSetMetadataExif(image, exif, exif_size) : AVIF_RESULT_OK;
    if (result == AVIF_RESULT_OK && qindex != 0) {
        char value[4];
        snprintf(value, sizeof value, "%d", qindex);
        result = avifEncoderSetCodecSpecificOption(encoder, "img-cq-qindex", value);
    }
    /* The still-image decoder preserves EXIF orientation but rejects container transforms. */
    image->transformFlags = AVIF_TRANSFORM_NONE;
    if (result == AVIF_RESULT_OK) result = avifImageRGBToYUV(image, &rgb);
    if (result == AVIF_RESULT_OK) result = avifEncoderAddImage(encoder, image, 1, AVIF_ADD_IMAGE_FLAG_SINGLE);
    if (result == AVIF_RESULT_OK) result = avifEncoderFinish(encoder, output);
    avifEncoderDestroy(encoder);
    avifImageDestroy(image);
    return result;
}
