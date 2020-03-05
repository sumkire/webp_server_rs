extern crate libc;
extern crate getopts;
extern crate serde;

use crossbeam_channel::tick;
use getopts::Options;
use glob::glob;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use image;
use libc::{size_t, c_int, c_uchar};
use num_cpus;
use serde::Deserialize;
use std::cmp::{max, min};
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::string::String;
use std::time::{Duration, SystemTime};
use threadpool::ThreadPool;
use tokio::fs;
use walkdir::WalkDir;

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
#[link(name = "webpwrapper", kind = "static")]
extern {
    fn new_webpwrapper_config() -> *const c_uchar;
    fn drop_webpwrapper_config(config: *const c_uchar);

    fn set_webp_config_preset(config: *const c_uchar, value: i32, quality_factor: f32);
    fn set_webp_config_alpha_compression(config: *const c_uchar, value: i32);
    fn set_webp_config_alpha_filtering(config: *const c_uchar, value: i32);
    fn set_webp_config_alpha_quality(config: *const c_uchar, value: i32);
    fn set_webp_config_autofilter(config: *const c_uchar, value: i32);
    fn set_webp_config_emulate_jpeg_size(config: *const c_uchar, value: i32);
    fn set_webp_config_exact(config: *const c_uchar, value: i32);
    fn set_webp_config_filter_sharpness(config: *const c_uchar, value: i32);
    fn set_webp_config_filter_strength(config: *const c_uchar, value: i32);
    fn set_webp_config_filter_type(config: *const c_uchar, value: i32);
    fn set_webp_config_image_hint(config: *const c_uchar, value: i32);
    fn set_webp_config_lossless(config: *const c_uchar, value: i32);
    fn set_webp_config_low_memory(config: *const c_uchar, value: i32);
    fn set_webp_config_method(config: *const c_uchar, value: i32);
    fn set_webp_config_near_lossless(config: *const c_uchar, value: i32);
    fn set_webp_config_partition_limit(config: *const c_uchar, value: i32);
    fn set_webp_config_partitions(config: *const c_uchar, value: i32);
    fn set_webp_config_pass(config: *const c_uchar, value: i32);
    fn set_webp_config_preprocessing(config: *const c_uchar, value: i32);
    fn set_webp_config_quality(config: *const c_uchar, value: f32);
    fn set_webp_config_segments(config: *const c_uchar, value: i32);
    fn set_webp_config_sns_strength(config: *const c_uchar, value: i32);
    fn set_webp_config_target_PSNR(config: *const c_uchar, value: f32);
    fn set_webp_config_target_size(config: *const c_uchar, value: i32);
    fn set_webp_config_thread_level(config: *const c_uchar, value: i32);
    fn set_webp_config_use_delta_palette(config: *const c_uchar, value: i32);
    fn set_webp_config_use_sharp_yuv(config: *const c_uchar, value: i32);

    fn webp_encoder(rgba: *const u8, width: c_int, height: c_int, stride: c_int,
                    importer: c_int,
                    config: *const c_uchar,
                    output: &*mut c_uchar
    ) -> size_t;
}

const fn config_default_3333u16() -> u16 { 3333 }
fn config_default_127_0_0_1() -> String { "127.0.0.1".to_string() }

#[derive(Deserialize, Debug, Clone)]
struct WebPServerConfig {
    #[serde(default = "config_default_127_0_0_1")]
    host: String,
    #[serde(default = "config_default_3333u16")]
    port: u16,
    img_path: String,
    webp_path: String,
    global_config: DirectoryLevelConfig
}

#[derive(Deserialize, Debug, Clone)]
struct DirectoryLevelConfig {
    lossless: Option<i32>,
    quality: Option<f32>,
    preset: Option<String>,
    method: Option<i32>,
    image_hint: Option<String>,
    target_size: Option<i32>,
    target_psnr: Option<f32>,
    segments: Option<i32>,
    sns_strength: Option<i32>,
    filter_strength: Option<i32>,
    filter_sharpness: Option<i32>,
    filter_type: Option<i32>,
    autofilter: Option<i32>,
    alpha_compression: Option<i32>,
    alpha_filtering: Option<i32>,
    alpha_quality: Option<i32>,
    pass: Option<i32>,
    preprocessing: Option<i32>,
    partitions: Option<i32>,
    partition_limit: Option<i32>,
    emulate_jpeg_size: Option<i32>,
    thread_level: Option<i32>,
    low_memory: Option<i32>,
    near_lossless: Option<i32>,
    exact: Option<i32>,
    use_delta_palette: Option<i32>,
    use_sharp_yuv: Option<i32>,
}

impl DirectoryLevelConfig {
    const fn new() -> DirectoryLevelConfig {
        DirectoryLevelConfig {
            lossless: None,
            quality: None,
            preset: None,
            method: None,
            image_hint: None,
            target_size: None,
            target_psnr: None,
            segments: None,
            sns_strength: None,
            filter_strength: None,
            filter_sharpness: None,
            filter_type: None,
            autofilter: None,
            alpha_compression: None,
            alpha_filtering: None,
            alpha_quality: None,
            pass: None,
            preprocessing: None,
            partitions: None,
            partition_limit: None,
            emulate_jpeg_size: None,
            thread_level: None,
            low_memory: None,
            near_lossless: None,
            exact: None,
            use_delta_palette: None,
            use_sharp_yuv: None
        }
    }

    fn detect(directory_path: &str, global_config: &DirectoryLevelConfig) -> DirectoryLevelConfig {
        let mut directory_level_config_path = PathBuf::from(directory_path);
        directory_level_config_path.push(".webp-conf");
        match std::fs::File::open(directory_level_config_path.as_path()) {
            Ok(file) => {
                match serde_json::from_reader(BufReader::new(file)) {
                    Ok(conf) => conf,
                    _ => global_config.clone(),
                }
            },
            _ => global_config.clone(),
        }
    }

    fn to_c_config_ptr(&self) -> *const c_uchar {
        unsafe {
            let ptr: *const c_uchar = new_webpwrapper_config();

            let preset = match &self.preset {
                Some(preset) => match &preset[..] {
                    "picture" => 2,
                    "photo" => 3,
                    "drawing" => 4,
                    "icon" => 5,
                    "text" => 6,
                    _ => 1,
                },
                _ => 1,
            };
            set_webp_config_preset(ptr, preset, self.quality.unwrap_or(75.0));

            macro_rules! set_parameter {
                ($func_name:ident, $config:expr, $config_ptr:expr, $param:ident) => {
                    if let Some(value) = $config.$param {
                        $func_name($config_ptr, value);
                    }
                };
            }

            set_parameter!(set_webp_config_alpha_compression, self, ptr, alpha_compression);
            set_parameter!(set_webp_config_alpha_filtering, self, ptr, alpha_filtering);
            set_parameter!(set_webp_config_alpha_quality, self, ptr, alpha_quality);
            set_parameter!(set_webp_config_autofilter, self, ptr, autofilter);
            set_parameter!(set_webp_config_emulate_jpeg_size, self, ptr, emulate_jpeg_size);
            set_parameter!(set_webp_config_exact, self, ptr, exact);
            set_parameter!(set_webp_config_filter_sharpness, self, ptr, filter_sharpness);
            set_parameter!(set_webp_config_filter_strength, self, ptr, filter_strength);
            set_parameter!(set_webp_config_filter_type, self, ptr, filter_type);
            set_webp_config_image_hint(ptr, match &self.image_hint {
                Some(hint) => match &hint[..] {
                    "picture" => 2,
                    "photo" => 3,
                    "graph" => 4,
                    _ => 1
                },
                _ => 1,
            });
            set_parameter!(set_webp_config_lossless, self, ptr, lossless);
            set_parameter!(set_webp_config_low_memory, self, ptr, low_memory);
            set_parameter!(set_webp_config_method, self, ptr, method);
            set_parameter!(set_webp_config_near_lossless, self, ptr, near_lossless);
            set_parameter!(set_webp_config_partition_limit, self, ptr, partition_limit);
            set_parameter!(set_webp_config_partitions, self, ptr, partitions);
            set_parameter!(set_webp_config_pass, self, ptr, pass);
            set_parameter!(set_webp_config_preprocessing, self, ptr, preprocessing);
            set_parameter!(set_webp_config_quality, self, ptr, quality);
            set_parameter!(set_webp_config_segments, self, ptr, segments);
            set_parameter!(set_webp_config_sns_strength, self, ptr, sns_strength);
            set_parameter!(set_webp_config_target_PSNR, self, ptr, target_psnr);
            set_parameter!(set_webp_config_target_size, self, ptr, target_size);
            set_parameter!(set_webp_config_thread_level, self, ptr, thread_level);
            set_parameter!(set_webp_config_use_delta_palette, self, ptr, use_delta_palette);
            set_parameter!(set_webp_config_use_sharp_yuv, self, ptr, use_sharp_yuv);
            ptr
        }
    }
}

#[derive(Debug, Clone)]
struct PrefetchConfig {
    enabled: bool,
    jobs: usize,
}

static mut PREFETCH: PrefetchConfig = PrefetchConfig { enabled: false, jobs: 1 };

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = get_server_listen_options();
    let server = Server::bind(&addr).serve(make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(webp_services)) }));
    prefetch_if_requested(from_cli_args(), true, ||{});
    println!("WebP image service on http://{}", addr);
    server.await?;
    Ok(())
}

fn prefetch_if_requested< Callback: 'static + std::marker::Send>(config: WebPServerConfig, verbose: bool, callback: Callback) where
    Callback: Fn() {
    let prefetch = unsafe { PREFETCH.clone() };

    if prefetch.enabled {
        let img_path = String::from(&config.img_path);
        let img_path_len = img_path.len();
        let webp_path = String::from(&config.webp_path);
        let global_config = config.global_config.clone();
        std::thread::spawn(move || {
            if verbose { println!("[INFO] Prefetch Started"); }
            let now = SystemTime::now();
            let mut filecount = 0usize;
            let pool = ThreadPool::new(prefetch.jobs);
            for entry in WalkDir::new(img_path).into_iter().filter_map(|e| e.ok()).filter(|e| e.path().is_file()) {
                let img_absolute_path = entry.path().to_path_buf();
                let img_uri_path = String::from(&entry.path().to_str().unwrap()[img_path_len..]);
                let webp_path_copy = webp_path.clone();
                filecount += 1;
                let global_config_copy = global_config.clone();
                pool.execute(move|| {
                    let webp_converted_paths = generate_webp_paths(&img_absolute_path, &img_uri_path, &webp_path_copy);
                    let webp_img_absolute_path = webp_converted_paths.0;

                    if !webp_img_absolute_path.exists() {
                        let webp_dir_absolute_path = webp_converted_paths.1;
                        let dir_absolute_path = webp_converted_paths.2.to_str().unwrap();

                        let global_config_copy = global_config_copy.clone();
                        let directory_level_config = if webp_dir_absolute_path.exists() {
                            DirectoryLevelConfig::detect(dir_absolute_path, &global_config_copy)
                        } else {
                            match std::fs::create_dir_all(&webp_dir_absolute_path) {
                                Err(_) => return,
                                _ => DirectoryLevelConfig::detect(dir_absolute_path, &global_config_copy),
                            }
                        };

                        // try to convert image to webp format
                        match convert(img_absolute_path.to_str().unwrap(), webp_img_absolute_path.to_str().unwrap(), &directory_level_config) {
                            Err(_) => (),
                            _ => remove_old_cached_webp(&webp_img_absolute_path, &webp_dir_absolute_path, &img_absolute_path),
                        };
                    }
                });
            }

            loop {
                let ticker = tick(Duration::from_micros(500));
                ticker.recv().unwrap();
                if pool.queued_count() == 0 && pool.active_count() == 0 {
                    if verbose { println!("\r[INFO] Prefetch progress: [{}/{}]\n[INFO] Prefetch done, elapsed time: {:.4} seconds", filecount, filecount, now.elapsed().unwrap().as_secs_f32()); }
                    callback();
                    return;
                } else {
                    if verbose { print!("\r[INFO] Prefetch progress: [{}/{}]", filecount - pool.queued_count() - pool.active_count(), filecount); }
                    let _ = std::io::stdout().flush();
                }
            }
        });
    }
}

fn generate_webp_paths(img_absolute_path: &PathBuf, img_uri_path: &str, webp_cache_path: &str) -> (PathBuf, PathBuf, PathBuf) {
    // aya.jpg
    let img_name = img_absolute_path.file_name().unwrap().to_str().unwrap();
    // /path/to
    let mut dir_uri_path = PathBuf::from(&img_uri_path);
    dir_uri_path.pop();
    let dir_uri_path = dir_uri_path.to_str().unwrap();
    // /IMG_PATH/path/to/
    let mut dir_absolute_path = PathBuf::from(&img_absolute_path);
    dir_absolute_path.pop();

    // 1582735380
    let modified_time = match std::fs::metadata(&img_absolute_path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified_time) => modified_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
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
    let mut webp_dir_absolute_path = PathBuf::from(webp_cache_path);
    webp_dir_absolute_path.push(&dir_uri_path[1..]);

    // /var/www/cache/path/to/aya.jpg.1582735380.webp
    let mut webp_img_absolute_path = PathBuf::from(&webp_dir_absolute_path);
    webp_img_absolute_path.push(&webp_img_name);

    (webp_img_absolute_path, webp_dir_absolute_path, dir_absolute_path)
}

fn remove_old_cached_webp(webp_img_absolute_path: &PathBuf, webp_dir_absolute_path: &PathBuf, img_absolute_path: &PathBuf) {
    // remove old webp files
    // /var/www/cache/path/to/aya.jpg.1582735300.webp <- older ones will be removed
    // /var/www/cache/path/to/aya.jpg.1582735380.webp <- keep the latest one
    let mut glob_pattern = String::from(webp_dir_absolute_path.canonicalize().unwrap().to_str().unwrap());
    glob_pattern.push_str(&format!("/{}.*.webp", img_absolute_path.file_name().unwrap().to_str().unwrap()));

    if let Ok(webp_img_absolute_path) = webp_img_absolute_path.canonicalize() {
        if let Some(webp_img_absolute_path) = webp_img_absolute_path.to_str() {
            for entry in glob(&glob_pattern).expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => if let Some(path) = path.to_str() {
                        if path != webp_img_absolute_path {
                            let _ = std::fs::remove_file(&path);
                        }
                    },
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
    }
}

async fn webp_services(req: Request<Body>) -> hyper::Result<Response<Body>> {
    if req.method() != hyper::Method::GET {
        Ok(method_not_allowed())
    } else {
        let config = from_cli_args();
        // /path/to/aya.jpg
        let img_uri_path = req.uri().path();
        // /IMG_PATH/path/to/aya.jpg
        let config_img_path = config.img_path.to_string();
        let mut img_absolute_path = PathBuf::from(&config_img_path);
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

        let webp_converted_paths = generate_webp_paths(&img_absolute_path, img_uri_path, &config.webp_path);
        let webp_img_absolute_path = webp_converted_paths.0;
        let webp_dir_absolute_path = webp_converted_paths.1;
        let dir_absolute_path = webp_converted_paths.2.to_str().unwrap();

        return if webp_img_absolute_path.exists() {
            Ok(sendfile!(webp_img_absolute_path.to_str().unwrap()))
        } else {
            // send original file if we cannot create cache directory or subdirectory
            let directory_level_config = match fs::create_dir_all(&webp_dir_absolute_path).await {
                Ok(()) => DirectoryLevelConfig::detect(dir_absolute_path, &config.global_config),
                Err(e) => {
                    eprintln!("{}", e);
                    return Ok(sendfile!(img_absolute_path.to_str().unwrap()));
                }
            };

            // try to convert image to webp format
            match convert(img_absolute_path.to_str().unwrap(), webp_img_absolute_path.to_str().unwrap(), &directory_level_config) {
                Err(e) => {
                    // send original file if failed
                    eprintln!("{}", e);
                    Ok(sendfile!(img_absolute_path.to_str().unwrap()))
                },
                _ => {
                    remove_old_cached_webp(&webp_img_absolute_path, &webp_dir_absolute_path, &img_absolute_path);
                    Ok(sendfile!(webp_img_absolute_path))
                },
            }
        }
    }
}

fn convert(original_file_path: &str, webp_file_path: &str, config: &DirectoryLevelConfig) -> Result<(), io::Error> {
    match image::open(original_file_path) {
        Ok(image) => {
            static WEBP_PICTURE_IMPORT_RGB: i32 = 1;
            static WEBP_PICTURE_IMPORT_RGBA: i32 = 2;
            static WEBP_PICTURE_IMPORT_BGR: i32 = 3;
            static WEBP_PICTURE_IMPORT_BGRA: i32 = 4;

            let encoded_size: size_t;
            let encoded_data: *mut c_uchar = null_mut();
            let config_c_ptr = config.to_c_config_ptr();

            match image {
                image::DynamicImage::ImageBgr8(image) => {
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 3;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                          WEBP_PICTURE_IMPORT_BGR, config_c_ptr, &encoded_data) };
                },
                image::DynamicImage::ImageRgb8(image) => {
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 3;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                                         WEBP_PICTURE_IMPORT_RGB, config_c_ptr,&encoded_data) };
                }
                image::DynamicImage::ImageBgra8(image) => {
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 4;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                                         WEBP_PICTURE_IMPORT_BGRA, config_c_ptr,&encoded_data) };
                },
                image::DynamicImage::ImageRgba8(image) => {
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 4;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                                         WEBP_PICTURE_IMPORT_BGRA, config_c_ptr,&encoded_data) };
                }
                image::DynamicImage::ImageRgb16(_) | image::DynamicImage::ImageLuma8(_) | image::DynamicImage::ImageLuma16(_) => {
                    let image = image.into_rgb();
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 3;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                                         WEBP_PICTURE_IMPORT_RGB, config_c_ptr,&encoded_data) };
                },
                image::DynamicImage::ImageRgba16(_) | image::DynamicImage::ImageLumaA8(_) | image::DynamicImage::ImageLumaA16(_) => {
                    let image = image.into_rgba();
                    let width = image.width() as i32;
                    let height = image.height() as i32;
                    let data = image.into_raw();
                    let data_ptr = data.as_ptr();
                    let stride = width * 4;
                    encoded_size = unsafe { webp_encoder(data_ptr, width, height, stride,
                                                         WEBP_PICTURE_IMPORT_RGBA,config_c_ptr,&encoded_data) };
                }
            };
            unsafe { drop_webpwrapper_config(config_c_ptr); };

            let encoded_data : Vec<u8> = unsafe { Vec::from_raw_parts(encoded_data, encoded_size, encoded_size) };
            let mut file = std::fs::File::create(&webp_file_path)?;
            file.write_all(&encoded_data)?;
            Ok(())
        },
        Err(e) => Err(std::io::Error::new( std::io::ErrorKind::InvalidData, format!("Cannot decode image: {}: {}", original_file_path, e) )),
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} -c CONF [options]", program);
    print!("{}", opts.usage(&brief));
}

fn from_cli_args() -> WebPServerConfig {
    static mut ONCE_TOKEN: bool = false;
    static mut CONFIG: WebPServerConfig = WebPServerConfig {
        host: String::new(),
        port: 0,
        img_path: String::new(),
        webp_path: String::new(),
        global_config: DirectoryLevelConfig::new(),
    };
    if unsafe { ONCE_TOKEN } {
        unsafe { CONFIG.clone() }
    } else {
        let args: Vec<String> = std::env::args().collect();
        let program = args[0].clone();

        let mut opts = Options::new();
        opts.optopt("c", "config", "path config file", "CONF");
        opts.optflag("p", "prefetch", "enable prefetch");
        opts.optopt("j", "jobs", "max threads for prefetch, [1, num_cpus]", "JOBS");
        opts.optflag("h", "help", "print usage");
        let matches = match opts.parse(&args[1..]) {
            Ok(m) => { m }
            Err(f) => { panic!(f.to_string()) }
        };

        if matches.opt_present("h") {
            print_usage(&program, opts);
        }

        if matches.opt_present("p") {
            unsafe { PREFETCH.enabled = true; PREFETCH.jobs = num_cpus::get(); };
            // cap jobs in [1, num_cpus]
            if let Some(jobs) = matches.opt_str("j") {
                unsafe { PREFETCH.jobs = min(max(1, jobs.parse::<usize>().unwrap_or(1)), num_cpus::get()); };
            }
        }

        let mut config_path = String::from("./config.json");
        if let Some(cli_config_path) = matches.opt_str("c") {
            config_path = cli_config_path.clone();
        }
        match load_config(config_path) {
            Ok(value) => {
                unsafe { CONFIG = value; CONFIG.clone() }
            },
            Err(e) => panic!("[ERROR] Cannot read config file {}", e),
        }
    }
}

fn load_config<P: AsRef<Path>>(conf_path: P) -> Result<WebPServerConfig, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(conf_path)?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

fn get_server_listen_options() -> SocketAddr {
    let config = from_cli_args();
    let host = config.host;
    format!("{}:{}", host, config.port).parse().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_webp_paths() {
        let webp_paths = generate_webp_paths(&PathBuf::from("./images/webp-server.jpg"), "/webp-server.jpg", "./cache");
        assert!(webp_paths.0.eq(&PathBuf::from(format!("./cache/webp-server.jpg.{}.webp", std::fs::metadata("./images/webp-server.jpg").unwrap().modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()))));
        assert!(webp_paths.1.eq(&PathBuf::from("./cache/")));
        assert!(webp_paths.2.eq(&PathBuf::from("./images")));
    }

    #[test]
    fn test_convert_mode_1() -> Result<(), io::Error> {
        let webp_paths = generate_webp_paths(&PathBuf::from("./images/lossless/webp-server.jpg"), "/lossless/webp-server.jpg", "./cache");

        // try to remove file before testing
        let _ = std::fs::remove_file(&webp_paths.0);
        assert!(!webp_paths.0.exists(),
                "Cannot remove old cached file at: {}", webp_paths.0.display());
        // and create corresponding directory
        let _ = std::fs::create_dir_all(&webp_paths.1);
        assert!(webp_paths.1.exists(),
                "Cannot create directory at: {}", webp_paths.1.display());

        let mut config = DirectoryLevelConfig::new();
        config.lossless = Some(1);
        config.near_lossless = Some(100);
        config.quality = Some(50.0);
        let _ = convert("images/lossless/webp-server.jpg", webp_paths.0.to_str().unwrap(), &config)?;
        assert!(webp_paths.0.exists(),
                "Converted WebP image should be at {}, but wasn't", webp_paths.0.display());
        assert_ne!(std::fs::metadata(&webp_paths.0).unwrap().len(), 0,
                   "Size of converted WebP image is 0, which suggested failed");
        let _ = std::fs::remove_file(webp_paths.0);
        Ok(())
    }

    #[test]
    fn test_convert_mode_2() -> Result<(), io::Error> {
        let webp_paths = generate_webp_paths(&PathBuf::from("./images/nearlossless/webp-server.jpg"), "/nearlossless/webp-server.jpg", "./cache");

        // try to remove file before testing
        let _ = std::fs::remove_file(&webp_paths.0);
        assert!(!webp_paths.0.exists(),
                "Cannot remove old cached file at: {}", webp_paths.0.display());
        // and create corresponding directory
        let _ = std::fs::create_dir_all(&webp_paths.1);
        assert!(webp_paths.1.exists(),
                "Cannot create directory at: {}", webp_paths.1.display());

        let mut config = DirectoryLevelConfig::new();
        config.lossless = Some(1);
        config.near_lossless = Some(50);
        config.quality = Some(40.0);
        convert("images/nearlossless/webp-server.jpg", webp_paths.0.to_str().unwrap(), &config)?;
        assert!(webp_paths.0.exists(),
                "Converted WebP image should be at {}, but wasn't", webp_paths.0.display());
        assert_ne!(std::fs::metadata(&webp_paths.0).unwrap().len(), 0,
                   "Size of converted WebP image is 0, which suggested failed");
        let _ = std::fs::remove_file(webp_paths.0);
        Ok(())
    }

    #[test]
    fn test_convert_mode_3() -> Result<(), io::Error> {
        let webp_paths = generate_webp_paths(&PathBuf::from("./images/lossy/webp-server.jpg"), "/lossy/webp-server.jpg", "./cache");

        // try to remove file before testing
        let _ = std::fs::remove_file(&webp_paths.0);
        assert!(!webp_paths.0.exists(),
                "Cannot remove old cached file at: {}", webp_paths.0.display());
        // and create corresponding directory
        let _ = std::fs::create_dir_all(&webp_paths.1);
        assert!(webp_paths.1.exists(),
                "Cannot create directory at: {}", webp_paths.1.display());

        let mut config = DirectoryLevelConfig::new();
        config.lossless = Some(0);
        config.near_lossless = Some(100);
        config.quality = Some(30.0);
        convert("images/lossy/webp-server.jpg", webp_paths.0.to_str().unwrap(), &config)?;
        assert!(webp_paths.0.exists(),
                "Converted WebP image should be at {}, but wasn't", webp_paths.0.display());
        assert_ne!(std::fs::metadata(&webp_paths.0).unwrap().len(), 0,
                   "Size of converted WebP image is 0, which suggested failed");
        let _ = std::fs::remove_file(webp_paths.0);
        Ok(())
    }

    fn generate_config(img_path: &str, webp_path: &str, lossless: i32, near_lossless: i32, quality: f32) -> WebPServerConfig {
        let mut config = WebPServerConfig {
            host: String::new(),
            port: 0,
            img_path: img_path.to_string(),
            webp_path: webp_path.to_string(),
            global_config: DirectoryLevelConfig::new(),
        };
        config.global_config.lossless = Some(lossless);
        config.global_config.near_lossless = Some(near_lossless);
        config.global_config.quality = Some(quality);
        config
    }

    #[test]
    fn test_prefetch() -> Result<(), io::Error> {
        // remove webp cache directory
        let prefetch_cache_path = "./prefetch-cache";
        let _ = std::fs::remove_dir_all(prefetch_cache_path);
        assert!(!PathBuf::from(prefetch_cache_path).exists());

        // enable prefetch
        unsafe { PREFETCH.enabled = true; };

        let done: std::sync::Arc<std::sync::atomic::AtomicBool> = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_copy = std::sync::Arc::clone(&done);
        prefetch_if_requested(generate_config("./images", prefetch_cache_path, 0, 100, 40.0), false, move ||{
            let prefetch_images = vec![
                "./images/webp-server.jpg",
                "./images/lossy/webp-server.jpg",
                "./images/nearlossless/webp-server.jpg",
                "./images/lossless/webp-server.jpg",
            ];

            for prefetch_image in prefetch_images {
                let webp_paths = generate_webp_paths(&PathBuf::from(prefetch_image), &prefetch_image[8..], "./prefetch-cache");
                if !webp_paths.0.exists() {
                    done_copy.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                assert!(webp_paths.0.exists(),
                        "Converted WebP image should be at {}, but wasn't", webp_paths.0.display());
                assert_ne!(std::fs::metadata(&webp_paths.0).unwrap().len(), 0,
                           "Size of converted WebP image is 0, which suggested failed");
            }

            let _ = std::fs::remove_dir_all(prefetch_cache_path);
            done_copy.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        loop {
            let ticker = tick(Duration::from_micros(500));
            ticker.recv().unwrap();
            if done.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
        }
        Ok(())
    }
}
