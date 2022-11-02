use log::{info, warn};
use serde::{Serialize, Deserialize};
use nokhwa::{Camera, CameraInfo, CameraFormat, Resolution, FrameFormat};
use std::path::{PathBuf};
use image::ImageFormat;
use image::imageops::crop_imm;
use chrono::{Local};

fn main() {
    simple_logger::init().unwrap();
    let config = get_config();

    let cameras = get_cameras();

    let camera_index = get_camera_index(&config, &cameras);

    let mut camera = get_camera(camera_index, &config);

    camera.open_stream().expect("Failed to open stream");
    let frame = camera.frame().expect("Failed to get frame");

    let image = crop_imm(&frame, config.crop_x, config.crop_y, config.crop_width, config.crop_height).to_image();

    let output_path = get_output_path(&config);
    image.save_with_format(output_path, ImageFormat::Jpeg).expect("Failed to save image.");
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            camera_id: "".to_string(),
            camera_width: 640,
            camera_height: 480,
            camera_frame_rate: 30,
            output_dir: "output".to_string(),
            output_prefix: "".to_string(),
            crop_x: 0,
            crop_y: 0,
            crop_width: 640,
            crop_height: 480,
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
