//
//  webpwrapper.c
//  webpwrapper
//
//  Created by Cocoa on 01/03/2020.
//  Copyright © 2020 Cocoa. All rights reserved.
//

#include <webp/encode.h>

typedef int (*Importer)(WebPPicture* const, const uint8_t* const, int);

size_t webp_encoder(const uint8_t* rgba, int width, int height, int stride,
                     Importer import, float quality_factor, int type,
                     uint8_t** output) {
  WebPPicture pic;
  WebPConfig config;
  WebPMemoryWriter wrt;
  int ok;

  if (output == NULL) return 0;

  if (!WebPConfigPreset(&config, WEBP_PRESET_DEFAULT, quality_factor) ||
      !WebPPictureInit(&pic)) {
    return 0;  // shouldn't happen, except if system installation is broken
  }

    if (type == 0) {
        config.lossless = 1;
        pic.use_argb = 1;
    } else if (type == 1) {
        config.lossless = 1;
        pic.use_argb = 1;
        config.near_lossless = quality_factor;
    } else {
        config.lossless = 0;
        pic.use_argb = 0;
        config.quality = quality_factor;
    }
  
  pic.width = width;
  pic.height = height;
  pic.writer = WebPMemoryWrite;
  pic.custom_ptr = &wrt;
  WebPMemoryWriterInit(&wrt);

  ok = import(&pic, rgba, stride) && WebPEncode(&config, &pic);
  WebPPictureFree(&pic);
  if (!ok) {
    WebPMemoryWriterClear(&wrt);
    *output = NULL;
    return 0;
  }
  *output = wrt.mem;
  return wrt.size;
}
