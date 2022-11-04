use log::{info, warn, error};
use serde::{Serialize, Deserialize};
use nokhwa::{Camera, CameraInfo, CameraFormat, Resolution, FrameFormat};
use std::path::{PathBuf};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use image::ImageFormat;
use image::imageops::crop_imm;
use chrono::{Local};
use s3::Region;
use s3::bucket::Bucket;
use awscreds::Credentials;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let config = get_config();

    let cameras = get_cameras();

    let camera_index = get_camera_index(&config, &cameras);

    let mut camera = get_camera(camera_index, &config);

    camera.open_stream().expect("Failed to open stream");
    let frame = camera.frame().expect("Failed to get frame");

    let image = crop_imm(&frame, config.crop_x, config.crop_y, config.crop_width, config.crop_height).to_image();

    let output_path = get_output_path(&config);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect(&format!("Could not create directory {:?}", parent));
    }
    image.save_with_format(&output_path, ImageFormat::Jpeg).expect("Failed to save picture");

    info!("Updating image.");
    let mut image_file = File::open(&output_path).expect("Failed to open file for upload");
    let mut image_file_buffer = Vec::new();
    image_file.read_to_end(&mut image_file_buffer).expect("Failed to read file for upload");
    let bucket = get_bucket(&config);
    bucket.put_object_with_content_type(
        format!("{}pictures/{}", config.r2_project_prefix, output_path.file_name().unwrap().to_str().unwrap()),
        &image_file_buffer,
        "image/jpeg",
    ).await.expect("Failed to upload picture");
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    camera_id: String,
    camera_width: u32,
    camera_height: u32,
    camera_frame_rate: u32,
    output_dir: String,
    output_prefix: String,
    crop_x: u32,
    crop_y: u32,
    crop_width: u32,
    crop_height: u32,
    no_default_camera: bool,
    r2_accound_id: String,
    r2_bucket_name: String,
    r2_access_key_id: String,
    r2_secret_access_key: String,
    r2_project_prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            camera_id: "".to_string(),
            camera_width: 640,
            camera_height: 480,
            camera_frame_rate: 30,
            output_dir: "pictures".to_string(),
            output_prefix: "".to_string(),
            crop_x: 0,
            crop_y: 0,
            crop_width: 640,
            crop_height: 480,
            no_default_camera: true,
            r2_accound_id: "".to_string(),
            r2_bucket_name: "".to_string(),
            r2_access_key_id: "".to_string(),
            r2_secret_access_key: "".to_string(),
            r2_project_prefix: "plant-cam/".to_string(),
        }
    }
}

fn get_config() -> Config {
    let cfg: Config = confy::load_path("config.toml").expect("Error with config file");
    info!("{:?}", cfg);
    cfg
}

fn get_cameras() -> Vec<CameraInfo> {
    let cameras = nokhwa::query_devices(nokhwa::CaptureAPIBackend::Auto).unwrap();
    info!("{} Cameras detected.", cameras.len());
    cameras
}

fn get_camera_index(config: &Config, cameras: &Vec<CameraInfo>) -> usize {
    for camera in cameras.iter() {
        if camera.misc().to_lowercase().contains(&config.camera_id.to_lowercase()) {
            info!("Using camera {} {}.", camera.index(), camera.human_name());
            return camera.index();
        }
    }
    if config.no_default_camera {
        error!("Could not find camera with id {}, exiting...", &config.camera_id);
        panic!("Could not find camera with id {}", &config.camera_id);
    }
    warn!("Could not find camera with id {}, using camera with index 0.", &config.camera_id);
    0
}

fn get_camera(index: usize, config: &Config) -> Camera {
    let camera = Camera::new(
        index,
        Some(CameraFormat::new(Resolution::new(config.camera_width, config.camera_height), FrameFormat::MJPEG, config.camera_frame_rate))
    ).expect("Failed to initialise camera");
    info!("Camera format: {}.", camera.camera_format());
    camera
}

fn get_output_path(config: &Config) -> PathBuf {
    let mut path = PathBuf::from(&config.output_dir);
    let mut filename = format!("{}.jpg", Local::now().format("%Y%m%d_%H%M"));
    if config.output_prefix != "" {
        filename = format!("{}-{}", config.output_prefix, filename);
    }
    path.push(filename);
    info!("Saving image to {:?}.", path);
    path
}

fn get_bucket(config: &Config) -> Bucket {
    Bucket::new(
        &config.r2_bucket_name,
        Region::R2 { account_id: config.r2_accound_id.to_owned() },
        Credentials::new(
            Some(&config.r2_access_key_id),
            Some(&config.r2_secret_access_key),
            None, None, None,
        ).expect("Could not initialise S3 credential"),
    ).expect("Could not instantiate the existing bucket")
}
