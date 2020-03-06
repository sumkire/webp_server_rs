libwebp :
	if [ ! -e "libwebp-1.1.0.tar.gz" ]; then curl https://codeload.github.com/webmproject/libwebp/tar.gz/v1.1.0 -o libwebp-1.1.0.tar.gz; fi
	if [ ! -e "libwebp-1.1.0" ]; then tar -xzf libwebp-1.1.0.tar.gz; fi
	@rm -rf libwebp-1.1.0/build
	mkdir -p libwebp-1.1.0/build
	cd libwebp-1.1.0/build && cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=../../deps -DWEBP_BUILD_CWEBP=OFF \
		-DWEBP_BUILD_DWEBP=OFF -DWEBP_BUILD_GIF2WEBP=OFF -DWEBP_BUILD_IMG2WEBP=OFF \
		-DWEBP_BUILD_VWEBP=OFF -DWEBP_BUILD_WEBPINFO=OFF -DWEBP_BUILD_WEBPMUX=OFF \
		-DWEBP_BUILD_EXTRAS=OFF -DWEBP_BUILD_WEBP_JS=OFF -DWEBP_BUILD_ANIM_UTILS=OFF \
		-DWEBP_NEAR_LOSSLESS=ON ..
	cd libwebp-1.1.0/build && make && make install

libwebpwrapper :
	@rm -rf webpwrapper/build
	mkdir -p webpwrapper/build
	cd webpwrapper/build && cmake .. && cmake --build . --config Release && cmake --install .

release :
	cargo build --release

debug :
	cargo build

test : release
	cargo test --release

deb : release
	cp -f target/release/webp-server-rs debian-package/usr/local/bin
	dpkg-deb -b debian-package
