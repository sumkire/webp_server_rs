# webp-server-rs ![Build Status](https://travis-ci.com/BlueCocoa/webp_server_rs.svg?branch=master)
Generate WebP image on-the-fly with Rust!

## Background

Speaking of switching to WebP image, at the first glance, I just did it with [a very naive approach](https://blog.0xbbc.com/2019/10/moving-to-webp-image-with-fallback-to-png/).

Then @Nova wrote a Node.JS server that can serve JPG/PNGs as WebP format on-the-fly. You can find that at [n0vad3v/webp_server](https://github.com/n0vad3v/webp_server).

A few days ago, @Nova and @Benny rewrite the WebP Server in Golang, [webp-sh/webp_server_go](https://github.com/webp-sh/webp_server_go). 

And that looks really promising, the size of the webp server, according to its description, had reduced from 43 MB to 15 MB, and it is a single binary instead of webp_server with node_modules.

I cloned that project and added [a tiny feature](https://github.com/webp-sh/webp_server_go/pull/2). However, I just found that although it is absolutely easy to implement the tiny feature, there is a potential design issue with the `fasthttp` module. In order to get everything work, it took me about 4 hours to debug on it.

Finally, it turned out to be a pointer of an internal variable (`ctx.Request.uri`, or so) was directly returned from `ctx.Path()`, and if users invoke `ctx.SendFile(filepath)`, the `ctx.Request.uri` will be set to `filepath`, which will also propagate to all variables that hold the shared value of `ctx.Path()`. You may visit my blog post for [details](https://blog.0xbbc.com/2020/02/note-about-encountered-memory-changes-for-no-reason-in-golang/).

## Now

Well, in aforementioned blog post, I said that it would be better if it was written in Rust. Now, let's make it come true and push the webp server even further.

### Comparison

#### Size

- `webp_server` with `node_modules`: 43M

|                                                              | Darwin x86_64 | Linux amd64 | Linux arm64 |
| ------------------------------------------------------------ | ------------- | ----------- | ----------- |
| [webp-server-go](https://github.com/webp-sh/webp_server_go)  | 15.4 MB       | 13.5 MB     | 10.5 MB     |
| [webp-server-rs](https://github.com/BlueCocoa/webp_server_rs) | 2.0 MB        | 2.5 MB      | 2.07 MB     |

#### Convenience

- webp_server: Clone the repo -> npm install -> run with pm2
- webp-server-go: Download a single binary -> Run
- webp-server-rs: Download a single binary -> Run

#### Performance

Not really tested. But IMHO it should be as fast as golang version.

#### Precision Control

webp-server-rust allows you set directory-level config and all libwebp parameters are available to you.

#### Supported Image Formats

webp-server-rust supports more image formats than webp-server-go.

Also GIF support is under consideration, currently webp-server-rust can only output the first frame of a GIF file. 

| Format | Converting |
| ------ | ---------- |
| PNG    | All supported color types |
| JPEG   | Baseline and progressive |
| BMP    | Yes |
| ICO    | Yes |
| TIFF   | Baseline(no fax support) + LZW + PackBits |
| PNM    | PBM, PGM, PPM, standard PAM |
| DDS    | DXT1, DXT3, DXT5 |

Please set proxy rules in Nginx / Apache configuration file to match specific types of files. [example](https://github.com/webp-sh/webp_server_rs#wordpress-example)

## Usage
Shamefully copy and paste most of the usage guidelines from [webp-sh/webp_server_go](https://github.com/webp-sh/webp_server_go), given that they are basically identical.

Regarding the `img_path` section in config.json. If you are serving images at https://example.com/images/aya.jpg and your files are at /var/www/site/images/aya.jpg, then `img_path` shall be `/var/www/site`.

### 1. Download or build the binary

Download the webp-server from [release](https://github.com/BlueCocoa/webp-server-rs/releases) page.

The `webp-server-rs-${ver}-linux-amd64.deb` package will ONLY INSTALL the binary to `/usr/local/bin/webp-server-rs`, the config file needs to be edited following the guideline below.

Wanna build your own binary? Check out [build](#build-your-own-binaries) section

### 2. config file

Create a config.json as follows to face your need.

```json
{
  "host": "127.0.0.1",
  "port": 3333,
  "img_path": "./images",
  "webp_path": "./cache",
  "global_config":  {
    "quality": 80
  }
}
```

Under `global_config` key, you can overwrite all parameters available from libwebp, which provides precise control to you. (also available in directory-level config)

#### Available Parameters
```
int lossless;           // Lossless encoding (0=lossy(default), 1=lossless).

float quality;          // between 0 and 100. For lossy, 0 gives the smallest
                        // size and 100 the largest. For lossless, this
                        // parameter is the amount of effort put into the
                        // compression: 0 is the fastest but gives larger
                        // files compared to the slowest, but best, 100.
                        
int method;             // quality/speed trade-off (0=fast, 6=slower-better)

string image_hint;      // Hint for image type (lossless only for now).
                        // - default: default preset.
                        // - picture: digital picture, like portrait, inner shot
                        // - photo:   outdoor photograph, with natural lighting
                        // - graph:   Discrete tone image (graph, map-tile etc).

int target_size;        // if non-zero, set the desired target size in bytes.
                        // Takes precedence over the 'compression' parameter.
                        
float target_psnr;      // if non-zero, specifies the minimal distortion to
                        // try to achieve. Takes precedence over target_size.
                        
int segments;           // maximum number of segments to use, in [1..4]
int sns_strength;       // Spatial Noise Shaping. 0=off, 100=maximum.
int filter_strength;    // range: [0 = off .. 100 = strongest]
int filter_sharpness;   // range: [0 = off .. 7 = least sharp]
int filter_type;        // filtering type: 0 = simple, 1 = strong (only used
                        // if filter_strength > 0 or autofilter > 0)
                        
int autofilter;         // Auto adjust filter's strength [0 = off, 1 = on]
int alpha_compression;  // Algorithm for encoding the alpha plane (0 = none,
                        // 1 = compressed with WebP lossless). Default is 1.
                        
int alpha_filtering;    // Predictive filtering method for alpha plane.
                        //  0: none, 1: fast, 2: best. Default if 1.
                        
int alpha_quality;      // Between 0 (smallest size) and 100 (lossless).
                        // Default is 100.
                        
int pass;               // number of entropy-analysis passes (in [1..10]).
                        
int preprocessing;      // preprocessing filter:
                        // 0=none, 1=segment-smooth, 2=pseudo-random dithering
                        
int partitions;         // log2(number of token partitions) in [0..3]. Default
                        // is set to 0 for easier progressive decoding.
                        
int partition_limit;    // quality degradation allowed to fit the 512k limit
                        // on prediction modes coding (0: no degradation,
                        // 100: maximum possible degradation).
                        
int emulate_jpeg_size;  // If true, compression parameters will be remapped
                        // to better match the expected output size from
                        // JPEG compression. Generally, the output size will
                        // be similar but the degradation will be lower.
                        
int thread_level;       // If non-zero, try and use multi-threaded encoding.
int low_memory;         // If set, reduce memory usage (but increase CPU use).

int near_lossless;      // Near lossless encoding [0 = max loss .. 100 = off(default)].
int exact;              // if non-zero, preserve the exact RGB values under
                        // transparent area. Otherwise, discard this invisible
                        // RGB information for better compression. The default
                        // value is 0.

int use_delta_palette;  // reserved for future lossless feature
int use_sharp_yuv;      // if needed, use sharp (and slow) RGB->YUV conversion
```

#### Directory-Level Config

By placing a `.webp-conf` in intented directories, you can control the encoding `mode` and `quality` applied on the images inside that directory (the directory-level config will NOT propagate to its subdirectories).

If there is no directory level config file (`.webp-conf`) in the directory, then parameters in `config.json` will be used.

For example, we have such file layout

```
images
├── lossless
│   ├── .webp-conf
│   └── webp-server.jpeg (480911 bytes)
├── lossy
│   ├── .webp-conf
│   └── webp-server.jpeg (480911 bytes)
├── nearlossless
│   ├── .webp-conf
│   └── webp-server.jpeg (480911 bytes)
└── webp-server.jpeg (480911 bytes)
```

And the config files,
`config.json`, lossy encoding, quality will be used to reduce the size of output image

```json
{
  "host": "127.0.0.1",
  "port": 3333,
  "img_path": "./images",
  "webp_path": "./cache",
  "global_config":  {
    "quality": 80
  }
}
```

`images/lossless/.webp-conf`, lossless encoding
```json
{
  "lossless": 1
}
```

`images/nearlossless/.webp-conf`, near lossless encoding, `quality` and `near_lossless` will be used to reduce the size of output image
```json
{
  "quality": 60,
  "near_lossless": 80
}
```

`images/lossy/.webp-conf`, lossy encoding, quality will be used to reduce the size of output image
```json
{
  "quality": 40
}
```

And corresponding WebP images will be generated based on aforementioned rules,

```
cache
├── lossless
│   └── webp-server.jpeg.1579413991.webp (1938270 bytes)
├── lossy
│   └── webp-server.jpeg.1579413991.webp (160502 bytes)
├── nearlossless
│   └── webp-server.jpeg.1579413991.webp (212022 bytes)
└── webp-server.jpeg.1579413991.webp (317612 bytes)
```

### 3. Run
#### 3.1 Without prefetch
Run the binary like this: 

```
./webp-server-rs -c /path/to/config.json
# or
./webp-server-rs --config /path/to/config.json
```

#### 3.2 With prefetch
To enable prefetch feature, using `-p`. 

**Prefetch will be ran in background, WebP image service will operate normally.**

```
./webp-server-rs -c /path/to/config.json -p 
# or
./webp-server-rs --config /path/to/config.json --prefetch
```

By default, this will use all logical CPUs available in the system. 

To set max allowed number of threads that prefetch can use, using `-j`.

```
./webp-server-rs -c /path/to/config.json -p -j 4 
# or
./webp-server-rs --config /path/to/config.json --prefetch --jobs 4 
```

#### screen or tmux

Use screen or tmux to avoid being terminated. Let's take screen for example

```bash
screen -S webp
./webp-server-rs --config /path/to/config.json
```

#### systemd

Don't worry, we've got you covered!

```bash
cp webp-image.service /lib/systemd/system/
systemctl daemon-reload
systemctl enable webp-image.service
systemctl start webp-image.service
```

This systemd service script will assume that the binary is located at `/usr/local/bin/webp-server-rs` and the config file is located at `/etc/webp-server-rs/config.json`. It also uses `/var/cache/webps` as working directory.

### 4. Nginx proxy_pass

Let Nginx to `proxy_pass http://localhost:3333/;`, and your `webp-server-rs` is on-the-fly

#### WordPress example

```
location ~* \.(png|jpg|jpeg)$ {
    proxy_pass http://127.0.0.1:3333;
}
```

### Build your own binaries
Install latest version of Rust, clone the repo, and then...

#### 1. install cmake and curl with apt or whatever package manager on your system
```bash
# debian
apt install cmake curl
```

#### 2. download and build libwebp
```bash
# macOS / Linux
make libwebp

# windows
curl https://codeload.github.com/webmproject/libwebp/tar.gz/v1.1.0 -o v1.1.0.tar.gz
tar -xzf v1.1.0.tar.gz
mkdir -p libwebp-1.1.0/build && cd libwebp-1.1.0/build
cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=../../deps \
  -DWEBP_BUILD_CWEBP=OFF -DWEBP_BUILD_DWEBP=OFF -DWEBP_BUILD_GIF2WEBP=OFF \
  -DWEBP_BUILD_IMG2WEBP=OFF -DWEBP_BUILD_VWEBP=OFF -DWEBP_BUILD_WEBPINFO=OFF \
  -DWEBP_BUILD_WEBPMUX=OFF -DWEBP_BUILD_EXTRAS=OFF -DWEBP_BUILD_WEBP_JS=OFF \
  -DWEBP_BUILD_ANIM_UTILS=OFF -DWEBP_NEAR_LOSSLESS=ON ..
cmake --build .
cmake --install .
cd ../..
```

#### 3. build static webpwrapper library
```bash
# macOS / Linux
make libwebpwrapper.a

## windows
cl -c webpwrapper.c -I./deps/include
lib webpwrapper.obj /out:lib/webpwrapper.lib
```

#### 4. build webp-server-rs
```bash
# binary will be located at `target/release/webp-server-rs`
cargo build --release

# test
cargo test --release
```

[![forthebadge](https://forthebadge.com/images/badges/contains-cat-gifs.svg)]()  [![forthebadge](https://forthebadge.com/images/badges/built-with-love.svg)]() 
