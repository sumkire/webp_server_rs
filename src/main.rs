#[macro_use]
extern crate lazy_static;
extern crate libc;

use libc::{size_t, c_int, c_uchar};
use std::io;
use std::io::prelude::*;
use std::string::String;
use image;
use std::ptr::null_mut;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::env;
use glob::glob;
use tokio::fs;

extern crate serde;

use serde::Deserialize;
use std::option::Option;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::ffi::c_void;
use std::os::raw::c_float;

macro_rules! generate_http_response_builder {
    ($status_code:expr, $body:expr) => {{
        Response::builder().status($status_code).body($body.into()).unwrap()
    }};
}

macro_rules! generate_http_response {
    ($func_name:ident, $status_code:expr, $body:expr) => {
        fn $func_name() -> Response<Body> {
            generate_http_response_builder!($status_code, $body)
        }
    };
}

generate_http_response!(not_found, StatusCode::NOT_FOUND, "Not Found");
generate_http_response!(method_not_allowed, StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed");

macro_rules! sendfile {
    ($filename:expr) => {{
        match fs::read($filename).await {
            Ok(buffer) => generate_http_response_builder!(StatusCode::OK, buffer),
            Err(_) => not_found(),
        }
    }};
}

#[link(name = "webp", kind = "static")]
extern {
    fn WebPPictureImportRGB(picture: *mut c_void, rgb: *const u8, rgb_stride: c_int) -> c_int;
    fn WebPPictureImportRGBA(picture: *mut c_void, rgb: *const u8, rgb_stride: c_int) -> c_int;
    fn WebPPictureImportBGR(picture: *mut c_void, rgb: *const u8, rgb_stride: c_int) -> c_int;
    fn WebPPictureImportBGRA(picture: *mut c_void, rgb: *const u8, rgb_stride: c_int) -> c_int;
}

#[link(name = "webpwrapper", kind = "static")]
extern {
    fn webp_encoder(rgba: *const u8, width: c_int, height: c_int, stride: c_int,
              importer: unsafe extern "C" fn(*mut c_void, *const u8, c_int) -> c_int,
              quality_factor: c_float, encode_type: c_int, output: &*mut c_uchar
    ) -> size_t;
}

macro_rules! encode_to_webp_image {
    ($using_importer:ident, $metadata:expr, $quality:expr, $lossless:expr, $output:expr) => {
        unsafe { webp_encoder($metadata.data_ptr, $metadata.width, $metadata.height, $metadata.stride,
                              $using_importer, $quality, $lossless, $output) }
    };
}

#[derive(Deserialize, Debug, Clone)]
struct WebPServerConfig {
    host: Option<String>,
    port: Option<u16>,
    img_path: String,
    quality: f32,
    mode: i32,
}

#[derive(Deserialize, Debug, Clone)]
struct DirectoryLevelConfig {
    quality: f32,
    mode: i32,
}

impl DirectoryLevelConfig {
    fn detect(directory_path: &str) -> DirectoryLevelConfig {
        let config = DirectoryLevelConfig { quality: CONFIG.quality,  mode: CONFIG.mode };
        let mut directory_level_config_path = PathBuf::from(directory_path);
        directory_level_config_path.push(".webp-conf");
        match std::fs::File::open(directory_level_config_path.as_path()) {
            Ok(file) => {
                match serde_json::from_reader(BufReader::new(file)) {
                    Ok(conf) => conf,
                    _ => config,
                }
            },
            _ => config,
        }
    }
}

lazy_static! {
    static ref CONFIG: WebPServerConfig = from_cli_args();
}

struct ImageMetadata {
    width: i32,
    height: i32,
    data_ptr: *const u8,
    stride: i32
}

macro_rules! get_image_metadata {
    ($image:expr, $components_per_pixel:expr) => {{
        let width = $image.width() as i32;
        let height = $image.height() as i32;
        let data_ptr = $image.into_raw().as_ptr();
        let stride = width * $components_per_pixel;
        ImageMetadata { width, height, data_ptr, stride }
    }};
}

#[tokio::main]
async fn main() {
    let (host, port) = get_server_listen_options();
    let addr = format!("{}:{}", host, port).parse().unwrap();
    let server = Server::bind(&addr).serve(make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(webp_services)) }));

    println!("WebP image service on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn webp_services(req: Request<Body>) -> hyper::Result<Response<Body>> {
    if req.method() != hyper::Method::GET {
        Ok(method_not_allowed())
    } else {
        // /path/to/aya.jpg
        let img_uri_path = req.uri().path();
        // /IMG_PATH/path/to/aya.jpg
        let mut img_absolute_path = PathBuf::from(&CONFIG.img_path);
        img_absolute_path.push(&img_uri_path[1..]);

        // Check the original image for existence and ensure its a file
        let original_img_exists = img_absolute_path.exists();
        if !original_img_exists || !img_absolute_path.is_file() {
            return Ok(not_found());
        }

        // Check for Safari users
        let is_safari = match req.headers().get("user-agent") {
            Some(ua) => match ua.to_str() {
                Ok(ua) => ua.contains("Safari") && !ua.contains("Chrome") && !ua.contains("Firefox"),
                Err(_) => false,
            },
            _ => false,
        };
        if is_safari {
            return Ok(sendfile!(img_absolute_path.to_str().unwrap()))
        }

        // aya.jpg
        let img_name = img_absolute_path.file_name().unwrap().to_str().unwrap();
        // /path/to
        let mut dir_uri_path = PathBuf::from(&img_uri_path);
        dir_uri_path.pop();
        let dir_uri_path = dir_uri_path.to_str().unwrap();
        // /IMG_PATH/path/to/
        let mut dir_absolute_path = PathBuf::from(&img_absolute_path);
        dir_absolute_path.pop();
        let dir_absolute_path = dir_absolute_path.to_str().unwrap();

        // /var/www
        let cwd = match env::current_dir() {
            Ok(cwd) => String::from(cwd.to_str().unwrap()),
            Err(e) => {
                eprintln!("{}", e);
                return Ok(sendfile!(img_absolute_path.to_str().unwrap()));
            }
        };

        // 1582735380
        let modified_time = match std::fs::metadata(&img_absolute_path) {
            Ok(metadata) => match metadata.modified() {
                Ok(modified_time) => modified_time.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                Err(e) => {
                    eprintln!("{}", e);
                    0
                }
            },
            Err(e) => {
                eprintln!("{}", e);
                0
            }
        };

        // aya.jpg.1582735380.webp
        let mut webp_img_name = String::from(img_name);
        webp_img_name.push('.');
        webp_img_name.push_str(&modified_time.to_string());
        webp_img_name.push_str(".webp");

        // /var/www/cache/path/to
        let mut webp_dir_absolute_path = PathBuf::from(&cwd);
        webp_dir_absolute_path.push("cache");
        webp_dir_absolute_path.push(&dir_uri_path[1..]);

        // /var/www/cache/path/to/aya.jpg.1582735380.webp
        let mut webp_img_absolute_path = PathBuf::from(&webp_dir_absolute_path);
        webp_img_absolute_path.push(&webp_img_name);

        if webp_img_absolute_path.exists() {
            return Ok(sendfile!(webp_img_absolute_path.to_str().unwrap()));
        } else {
            // send original file if we cannot create cache directory or subdirectory
            match fs::create_dir_all(&webp_dir_absolute_path).await {
                Ok(()) => (),
                Err(e) => {
                    eprintln!("{}", e);
                    return Ok(sendfile!(img_absolute_path.to_str().unwrap()));
                }
            };

            let directory_level_config = DirectoryLevelConfig::detect(dir_absolute_path);

            // try to convert image to webp format
            return match convert(img_absolute_path.to_str().unwrap(), webp_img_absolute_path.to_str().unwrap(), directory_level_config.quality, directory_level_config.mode) {
                Err(e) => {
                    // send original file if failed
                    eprintln!("{}", e);
                    Ok(sendfile!(img_absolute_path.to_str().unwrap()))
                },
                _ => {
                    // remove old webp files
                    // /var/www/cache/path/to/aya.jpg.1582735300.webp <- older ones will be removed
                    // /var/www/cache/path/to/aya.jpg.1582735380.webp <- keep the latest one
                    let mut glob_pattern = String::from(webp_dir_absolute_path.to_str().unwrap());
                    glob_pattern.push_str(&format!("{}.*.webp", img_name));

                    let webp_img_absolute_path = webp_img_absolute_path.to_str().unwrap();
                    for entry in glob(&glob_pattern).expect("Failed to read glob pattern") {
                        match entry {
                            Ok(path) => if path.to_str().unwrap() != webp_img_absolute_path {
                                let _ = std::fs::remove_file(&path);
                            },
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    
                    // send webp file
                    Ok(sendfile!(webp_img_absolute_path))
                },
            };
        }
    }
}

fn convert(original_file_path: &str, webp_file_path: &str, quality: f32, mode: i32) -> Result<(), io::Error> {
    let mut file = std::fs::File::open(original_file_path)?;
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;

    match image::load_from_memory(&buffer) {
        Ok(image) => {
            let metadata;
            let encoded_size: size_t;
            let encoded_data: *mut c_uchar = null_mut();

            match image {
                image::DynamicImage::ImageBgr8(image) => {
                    metadata = get_image_metadata!(image, 3);
                    encoded_size = encode_to_webp_image!(WebPPictureImportBGR, metadata, quality, mode, &encoded_data);
                },
                image::DynamicImage::ImageRgb8(image) => {
                    metadata = get_image_metadata!(image, 3);
                    encoded_size = encode_to_webp_image!(WebPPictureImportRGB, metadata, quality, mode, &encoded_data);
                }
                image::DynamicImage::ImageBgra8(image) => {
                    metadata = get_image_metadata!(image, 4);
                    encoded_size = encode_to_webp_image!(WebPPictureImportBGRA, metadata, quality, mode, &encoded_data);
                },
                image::DynamicImage::ImageRgba8(image) => {
                    metadata = get_image_metadata!(image, 4);
                    encoded_size = encode_to_webp_image!(WebPPictureImportRGBA, metadata, quality, mode, &encoded_data);
                }
                image::DynamicImage::ImageRgb16(_) | image::DynamicImage::ImageLuma8(_) | image::DynamicImage::ImageLuma16(_) => {
                    metadata = get_image_metadata!(image.to_rgb(), 3);
                    encoded_size = encode_to_webp_image!(WebPPictureImportRGB, metadata, quality, mode, &encoded_data);
                },
                image::DynamicImage::ImageRgba16(_) | image::DynamicImage::ImageLumaA8(_) | image::DynamicImage::ImageLumaA16(_) => {
                    metadata = get_image_metadata!(image.to_rgba(), 4);
                    encoded_size = encode_to_webp_image!(WebPPictureImportRGBA, metadata, quality, mode, &encoded_data);
                }
            };

            let encoded_data : Vec<u8> = unsafe { Vec::from_raw_parts(encoded_data, encoded_size, encoded_size) };
            let mut file = std::fs::File::create(&webp_file_path)?;
            file.write_all(&encoded_data)?;
        },
        Err(e) => {
            eprintln!("{}", e);
            return Err(std::io::Error::new( std::io::ErrorKind::InvalidData, format!("Cannot decode image: {}", original_file_path) ));
        },
    };

    Ok(())
}

fn from_cli_args() -> WebPServerConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = "./config.json";
    if args.len() >= 2 {
        config_path = &*args[1];
    }
    match load_config(config_path) {
        Ok(value) => value,
        Err(e) => panic!("[ERROR] Cannot read config file {}", e),
    }
}

fn load_config<P: AsRef<Path>>(conf_path: P) -> Result<WebPServerConfig, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(conf_path)?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

fn get_server_listen_options() -> (String, u16) {
    let mut host = String::from("127.0.0.1");
    if let Some(custom_host) = &CONFIG.host {
        host = String::from(custom_host);
    }
    let port: u16 = match &CONFIG.port {
        Some(port) => *port,
        _ => 3333,
    };
    (host, port)
}
