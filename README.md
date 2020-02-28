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
- `webp-server(go)` single binary: 15M
- `webp_server(rust)` single binary: 3.6M(macOS) / 6.4M(Linux)

#### Convenience

- webp_server: Clone the repo -> npm install -> run with pm2
- webp-server(go): Download a single binary -> Run
- webp-server(rust): Download a single binary -> Run

#### Performance

Not really tested. But IMHO it should be as fast as golang version.

### Supported Image Formats

| Format | Converting | Default On |
| ------ | ---------- | ---------- |
| PNG    | All supported color types | Yes |
| JPEG   | Baseline and progressive | Yes |
| BMP    | Yes | No |
| ICO    | Yes | No |
| TIFF   | Baseline(no fax support) + LZW + PackBits | No |
| PNM    | PBM, PGM, PPM, standard PAM | No |
| DDS    | DXT1, DXT3, DXT5 | No |

Currently, only image with one of RGB8 / BGR8 / RGBA8 / BGRA8 colorspace will be convert to WebP image. 

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
    "img_path": "/path/to/images",
    "allowed_types": ["jpg","png","jpeg"]
}
```

### 3. Run
Run the binary like this: `./webp-server-rs /path/to/config.json`

#### screen or tmux

Use screen or tmux to avoid being terminated. Let's take screen for example

```bash
screen -S webp
./webp-server-rs /path/to/config.json
```

#### systemd

Don't worry, we've got you covered!

```bash
cp webp-image.service /lib/systemd/systemd/
systemctl daemon-reload
systemctl enable webp-image.service
systemctl start webp-image.service
```

This systemd service script will assume that the binary is located at `/usr/local/bin/webp-server-rs` and the config file is located at `/etc/webp-server-rs/config.json`. It also uses `/var/cache/webps` as working directory.

### 4. Nginx proxy_pass

Let Nginx to `proxy_pass http://localhost:3333/;`, and your `webp-server-rs` is on-the-fly

#### WordPress example

```
location ^~ /wp-content/uploads/ {
    proxy_pass http://127.0.0.1:3333;
}
```

### Build your own binaries
Install latest version of Rust, clone the repo, and then...

```bash
# install cmake and unzip with apt or whatever package manager on your system
apt install cmake unzip

# download and build libwebp
wget https://github.com/webmproject/libwebp/archive/v1.1.0.zip -O v1.1.0.zip
unzip v1.1.0.zip
mkdir -p libwebp-1.1.0/build && pushd libwebp-1.1.0/build
cmake -D CMAKE_BUILD_TYPE=Release ..
sudo make install
popd

# build webp-server-rs
cd webp_server_rs
cargo build --release
```

[![forthebadge](https://forthebadge.com/images/badges/contains-cat-gifs.svg)]()  [![forthebadge](https://forthebadge.com/images/badges/built-with-love.svg)]() 
