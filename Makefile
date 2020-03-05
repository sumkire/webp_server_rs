libwebpwrapper.a :
	@rm -f lib/libwebpwrapper.a
	cc -c -fPIC -Os -I./deps/include webpwrapper.c -o webpwrapper.o
	ar rcs lib/libwebpwrapper.a webpwrapper.o

libwebp :
	curl https://codeload.github.com/webmproject/libwebp/tar.gz/v1.1.0 -o v1.1.0.tar.gz
	tar -xzf v1.1.0.tar.gz
	mkdir -p libwebp-1.1.0/build && pushd libwebp-1.1.0/build
	cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=../../deps ..
	make
	make install
	popd

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
