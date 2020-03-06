cmake_minimum_required(VERSION 2.8.12)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "Choose the type of build" FORCE)
endif()

project(webpwrapper)

##############################################

# Build the webpwrapper library.
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Os -fPIC")
include_directories("${CMAKE_SOURCE_DIR}/../deps/include")
add_library(webpwrapper STATIC "webpwrapper.c")
install(TARGETS webpwrapper DESTINATION "${CMAKE_SOURCE_DIR}/../deps/lib")
