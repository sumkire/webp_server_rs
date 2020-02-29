libwebpwrapper.a :
	cc -c -fPIC -Os webpwrapper.c -o webpwrapper.o
	ar rcs lib/libwebpwrapper.a webpwrapper.o

clean :
	@rm -f lib/libwebpwrapper.a webpwrapper.o
