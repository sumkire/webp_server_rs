//
//  webpwrapper.c
//  webpwrapper
//
//  Created by Cocoa on 01/03/2020.
//  Copyright © 2020 Cocoa. All rights reserved.
//

#include <stdlib.h>
#include <string.h>
#include <webp/encode.h>

typedef int (*Importer)(WebPPicture* const, const uint8_t* const, int);

WebPConfig * new_webpwrapper_config() {
  WebPConfig * config = (WebPConfig *)malloc(sizeof(struct WebPConfig));
  memset((void *)config, 0, sizeof(struct WebPConfig));
  return config;
}

void drop_webpwrapper_config(WebPConfig * config) {
  free((void *)config);
}

#define SET_WEBP_CONFIG_PARAM_INT(parameter_name) \
void set_webp_config_##parameter_name (WebPConfig * config, int value) {\
  config->parameter_name = value;\
}

#define SET_WEBP_CONFIG_PARAM_FLOAT(parameter_name) \
void set_webp_config_##parameter_name(WebPConfig * config, float value) {\
  config->parameter_name = value;\
}

SET_WEBP_CONFIG_PARAM_INT(lossless)
SET_WEBP_CONFIG_PARAM_FLOAT(quality)
SET_WEBP_CONFIG_PARAM_INT(method)
SET_WEBP_CONFIG_PARAM_INT(target_size)
SET_WEBP_CONFIG_PARAM_FLOAT(target_PSNR)
SET_WEBP_CONFIG_PARAM_INT(segments)
SET_WEBP_CONFIG_PARAM_INT(sns_strength)
SET_WEBP_CONFIG_PARAM_INT(filter_strength)
SET_WEBP_CONFIG_PARAM_INT(filter_sharpness)
SET_WEBP_CONFIG_PARAM_INT(filter_type)
SET_WEBP_CONFIG_PARAM_INT(autofilter)
SET_WEBP_CONFIG_PARAM_INT(alpha_compression)
SET_WEBP_CONFIG_PARAM_INT(alpha_filtering)
SET_WEBP_CONFIG_PARAM_INT(alpha_quality)
SET_WEBP_CONFIG_PARAM_INT(pass)
SET_WEBP_CONFIG_PARAM_INT(preprocessing)
SET_WEBP_CONFIG_PARAM_INT(partitions)
SET_WEBP_CONFIG_PARAM_INT(partition_limit)
SET_WEBP_CONFIG_PARAM_INT(emulate_jpeg_size)
SET_WEBP_CONFIG_PARAM_INT(thread_level)
SET_WEBP_CONFIG_PARAM_INT(low_memory)
SET_WEBP_CONFIG_PARAM_INT(near_lossless)
SET_WEBP_CONFIG_PARAM_INT(exact)
SET_WEBP_CONFIG_PARAM_INT(use_delta_palette)
SET_WEBP_CONFIG_PARAM_INT(use_sharp_yuv)

#define WEBP_HINT_DEFAULT_TYPE  1
#define WEBP_HINT_PICTURE_TYPE  2
#define WEBP_HINT_PHOTO_TYPE    3
#define WEBP_HINT_GRAPH_TYPE    4

void set_webp_config_image_hint(WebPConfig * config, int value) {
  switch (value) {
    case WEBP_HINT_DEFAULT_TYPE:
      config->image_hint = WEBP_HINT_DEFAULT;
      break;
    case WEBP_HINT_PICTURE_TYPE:
      config->image_hint = WEBP_HINT_PICTURE;
      break;
    case WEBP_HINT_PHOTO_TYPE:
      config->image_hint = WEBP_HINT_PHOTO;
      break;
    case WEBP_HINT_GRAPH_TYPE:
      config->image_hint = WEBP_HINT_GRAPH;
      break;
    default:
      break;
  }
}

#define WEBP_PRESET_DEFAULT_TYPE  1
#define WEBP_PRESET_PICTURE_TYPE  2
#define WEBP_PRESET_PHOTO_TYPE    3
#define WEBP_PRESET_DRAWING_TYPE  4
#define WEBP_PRESET_ICON_TYPE     5
#define WEBP_PRESET_TEXT_TYPE     6

void set_webp_config_preset(WebPConfig * config, int value, float quality_factor) {
  WebPPreset preset = WEBP_PRESET_DEFAULT;
  switch (value) {
    case WEBP_PRESET_DEFAULT_TYPE:
      preset = WEBP_PRESET_DEFAULT;
      break;
    case WEBP_PRESET_PICTURE_TYPE:
      preset = WEBP_PRESET_PICTURE;
      break;
    case WEBP_PRESET_PHOTO_TYPE:
      preset = WEBP_PRESET_PHOTO;
      break;
    case WEBP_PRESET_DRAWING_TYPE:
      preset = WEBP_PRESET_DRAWING;
      break;
    case WEBP_PRESET_ICON_TYPE:
      preset = WEBP_PRESET_ICON;
      break;
    case WEBP_PRESET_TEXT_TYPE:
      preset = WEBP_PRESET_TEXT;
      break;
    default:
      break;
  }
  WebPConfigPreset(config, preset, quality_factor);
}

#define WEBP_PICTURE_IMPORT_RGB_TYPE  1
#define WEBP_PICTURE_IMPORT_RGBA_TYPE 2
#define WEBP_PICTURE_IMPORT_BGR_TYPE  3
#define WEBP_PICTURE_IMPORT_BGRA_TYPE 4

size_t webp_encoder(const uint8_t* rgba, int width, int height, int stride,
                    int importer_type,
                    WebPConfig * config,
                    uint8_t** output) {
  WebPPicture pic;
  WebPMemoryWriter wrt;
  int ok;

  if (output == NULL) return 0;
  if (!WebPPictureInit(&pic)) return 0;

  Importer import = 0;
  switch (importer_type) {
  	case WEBP_PICTURE_IMPORT_RGB_TYPE:
      import = WebPPictureImportRGB;
      break;
    case WEBP_PICTURE_IMPORT_RGBA_TYPE:
      import = WebPPictureImportRGBA;
      break;
    case WEBP_PICTURE_IMPORT_BGR_TYPE:
      import = WebPPictureImportBGR;
      break;
    case WEBP_PICTURE_IMPORT_BGRA_TYPE:
      import = WebPPictureImportBGRA;
      break;
    default: {
      *output = NULL;
      return 0;
      break;
    }
  }
  
  pic.use_argb = !!config->lossless;
  pic.width = width;
  pic.height = height;
  pic.writer = WebPMemoryWrite;
  pic.custom_ptr = &wrt;
  WebPMemoryWriterInit(&wrt);

  ok = import(&pic, rgba, stride) && WebPEncode(config, &pic);
  WebPPictureFree(&pic);
  if (!ok) {
    WebPMemoryWriterClear(&wrt);
    *output = NULL;
    return 0;
  }
  *output = wrt.mem;
  return wrt.size;
}
