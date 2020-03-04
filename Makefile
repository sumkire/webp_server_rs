libwebp :
	wget https://github.com/webmproject/libwebp/archive/v1.1.0.zip -O v1.1.0.zip
	unzip v1.1.0.zip
	mkdir -p libwebp-1.1.0/build && pushd libwebp-1.1.0/build
	cmake -D CMAKE_BUILD_TYPE=Release ..
	make
	sudo make install
	popd

libwebpwrapper.a :
	@rm -f lib/libwebpwrapper.a
	cc -c -fPIC -Os webpwrapper.c -o webpwrapper.o
	ar rcs lib/libwebpwrapper.a webpwrapper.o

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
