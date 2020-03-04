# webp-server-rs ![Build Status](https://travis-ci.com/BlueCocoa/webp_server_rs.svg?branch=master)
Generate WebP image on-the-fly with Rust!

THIS PROJECT IS WORKING IN PROGRESS, DON'T USE IT IN PRODUCTION ENVIRONMENT.

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

|                                                              | Darwin x86_64 | Linux amd64 |
| ------------------------------------------------------------ | ------------- | ----------- |
| [webp-server-go](https://github.com/webp-sh/webp_server_go)  | 15.4MB        | 13.5MB      |
| [webp-server-rs](https://github.com/BlueCocoa/webp_server_rs) | 2.01MB        | 2.5MB       |

#### Convenience

- webp_server: Clone the repo -> npm install -> run with pm2
- webp-server(go): Download a single binary -> Run
- webp-server(rust): Download a single binary -> Run

#### Performance

Not really tested. But IMHO it should be as fast as golang version.

### Supported Image Formats

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
    "allowed_types": ["jpg","png","jpeg"],
    "quality": 90,
    "mode": 1
}
```

There are 3 possible values for `mode`,

- `1` stands for `lossless`, `quality` parameter will be ignored
- `2` stands for `near lossless`, `quality` parameter will be used to reduce the size of output image
- `3` stands for `lossy`, `quality` parameter will be used to reduce the size of output image

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
`config.json`, `mode = 2` => near lossless encoding, quality will be used to reduce the size of output image

```json
{
    "host": "127.0.0.1",
    "port": 3333,
    "img_path": "./images",
    "quality": 70,
    "mode": 2
}
```

`images/lossless/.webp-conf`, `mode = 1` => lossless encoding, quality will be ignored (but still required in the config file)
```json
{
    "mode": 1,
    "quality": 90
}
```

`images/nearlossless/.webp-conf`, `mode = 2` => near lossless encoding, quality will be used to reduce the size of output image
```json
{
    "mode": 2,
    "quality": 90
}
```

`images/lossy/.webp-conf`, `mode = 3` => lossy encoding, quality will be used to reduce the size of output image
```json
{
    "mode": 3,
    "quality": 40
}
```

And corresponding WebP images will be generated based on aforementioned rules,

```
cache
├── lossless
│   └── webp-server.jpeg.1579413991.webp (1647444 bytes)
├── lossy
│   └── webp-server.jpeg.1579413991.webp (160502 bytes)
├── nearlossless
│   └── webp-server.jpeg.1579413991.webp (497342 bytes)
└── webp-server.jpeg.1579413991.webp (242808 bytes)
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

```bash
# install cmake and curl with apt or whatever package manager on your system
apt install cmake curl

# download and build libwebp
make libwebp
## or you can run the script in the `Makefile` for `libwebp` target by yourself
curl https://codeload.github.com/webmproject/libwebp/tar.gz/v1.1.0 -o v1.1.0.tar.gz
tar -xzf v1.1.0.tar.gz
mkdir -p libwebp-1.1.0/build && pushd libwebp-1.1.0/build
cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=../../deps \
      -DWEBP_BUILD_CWEBP=OFF -DWEBP_BUILD_DWEBP=OFF -DWEBP_BUILD_GIF2WEBP=OFF \
      -DWEBP_BUILD_IMG2WEBP=OFF -DWEBP_BUILD_VWEBP=OFF -DWEBP_BUILD_WEBPINFO=OFF \
      -DWEBP_BUILD_WEBPMUX=OFF -DWEBP_BUILD_EXTRAS=OFF -DWEBP_BUILD_WEBP_JS=OFF \
      -DWEBP_BUILD_ANIM_UTILS=OFF -DWEBP_NEAR_LOSSLESS=ON ..
make
make install
popd

# build webp-server-rs
cd webp_server_rs
## for release version
make release
## for debug version
make debug

# build debian package
## requires dpkg-deb
make deb

# binary will be located at `target/release/webp-server-rs`
```

[![forthebadge](https://forthebadge.com/images/badges/contains-cat-gifs.svg)]()  [![forthebadge](https://forthebadge.com/images/badges/built-with-love.svg)]() 
