libwebpwrapper.a :
	@rm -f lib/libwebpwrapper.a
	cc -c -fPIC -Os -I./deps/include webpwrapper.c -o webpwrapper.o
	ar rcs lib/libwebpwrapper.a webpwrapper.o

libwebp :
	curl https://codeload.github.com/webmproject/libwebp/tar.gz/v1.1.0 -o v1.1.0.tar.gz
	tar -xzf v1.1.0.tar.gz
	mkdir -p libwebp-1.1.0/build && cd libwebp-1.1.0/build
	cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=../../deps -DWEBP_BUILD_CWEBP=OFF -DWEBP_BUILD_DWEBP=OFF -DWEBP_BUILD_GIF2WEBP=OFF -DWEBP_BUILD_IMG2WEBP=OFF -DWEBP_BUILD_VWEBP=OFF -DWEBP_BUILD_WEBPINFO=OFF -DWEBP_BUILD_WEBPMUX=OFF -DWEBP_BUILD_EXTRAS=OFF -DWEBP_BUILD_WEBP_JS=OFF -DWEBP_BUILD_ANIM_UTILS=OFF -DWEBP_NEAR_LOSSLESS=ON ..
	make
	make install
	cd ../..

release : libwebpwrapper.a
	cargo build --release

debug : libwebpwrapper.a
	cargo build

test : release
	cargo test --release

deb : release
	cp -f target/release/webp-server-rs debian-package/usr/local/bin
	dpkg-deb -b debian-package

clean :
	@rm -f lib/libwebpwrapper.a webpwrapper.o
