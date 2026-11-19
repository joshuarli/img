#include <avif/avif.h>
#include <stdint.h>
#include <stdlib.h>

int img_avif_encode(uint32_t width, uint32_t height, const uint8_t *pixels, int speed, int quality,
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
    encoder->quality = quality;
    encoder->maxThreads = 1;
    avifResult result = exif ? avifImageSetMetadataExif(image, exif, exif_size) : AVIF_RESULT_OK;
    /* The still-image decoder preserves EXIF orientation but rejects container transforms. */
    image->transformFlags = AVIF_TRANSFORM_NONE;
    if (result == AVIF_RESULT_OK) result = avifImageRGBToYUV(image, &rgb);
    if (result == AVIF_RESULT_OK) result = avifEncoderAddImage(encoder, image, 1, AVIF_ADD_IMAGE_FLAG_SINGLE);
    if (result == AVIF_RESULT_OK) result = avifEncoderFinish(encoder, output);
    avifEncoderDestroy(encoder);
    avifImageDestroy(image);
    return result;
}
