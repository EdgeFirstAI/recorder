//! MCAP Reader Library for Rust
//!
//! The Reader Library provides a mechanism for reading MCAP
//! frames and strcture the data in a readable format.

use anyhow::{Context, Result};
use camino::Utf8Path;
use edgefirst_schemas::{
    edgefirst_msgs::RadarCube,
    foxglove_msgs::{FoxgloveCompressedVideo, FoxgloveImageAnnotations},
    sensor_msgs::{point_field::FLOAT32, CameraInfo, Image, NavSatFix, PointCloud2, IMU},
    std_msgs::Header,
};
use memmap::Mmap;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::fmt;
use std::{
    fs,
    io::{Error, ErrorKind},
    result::Result::Ok,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub noise: f32,
    pub radial_speed: f32,
    pub snr: f32,
    pub power: f32,
    pub rcs: f32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {}",
            self.x, self.y, self.z, self.radial_speed, self.rcs, self.noise, self.power,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Imu {
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    velocity_x: f64,
    velocity_y: f64,
    velocity_z: f64,
    pub acceleration_x: f64,
    pub acceleration_y: f64,
    pub acceleration_z: f64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Gps {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Boxes3d {
    pub label: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
    pub l: f32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Boxes2d {
    pub label: i32,
    pub x: f32,
    pub y: f32,
    pub distance: f32,
    pub w: f32,
    pub h: f32,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Object {
    pub label: String,
    pub label_id: i16,
    pub sublabel: String,
    pub confidence: f32,
    pub position: [f32; 3],
    pub position_covariance: [f32; 6],
    pub velocity: [f32; 3],
    pub tracking_available: bool,
    pub tracking_state: i8,
    pub action_state: i8,
    pub bounding_box_2d: BoundingBox2Di,
    pub bounding_box_3d: BoundingBox3D,
    pub dimensions_3d: [f32; 3],
    pub skeleton_available: bool,
    pub body_format: i8,
    pub head_bounding_box_2d: BoundingBox2Df,
    pub head_bounding_box_3d: BoundingBox3D,
    pub head_position: [f32; 3],
    pub skeleton_2d: Skeleton2D,
    pub skeleton_3d: Skeleton3D,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoundingBox2Df {
    pub corners: [Keypoint2Df; 4],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoundingBox2Di {
    pub corners: [Keypoint2Di; 4],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoundingBox3D {
    pub corners: [Keypoint3D; 8],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Keypoint2Df {
    pub kp: [f32; 2],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Keypoint2Di {
    pub kp: [u32; 2],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Keypoint3D {
    pub kp: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skeleton2D {
    #[serde(with = "BigArray")]
    pub kp: [Keypoint2Df; 70],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skeleton3D {
    #[serde(with = "BigArray")]
    pub kp: [Keypoint3D; 70],
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ObjectsStamped {
    pub header: Header,
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cube {
    pub layout: Vec<u8>,
    pub shape: Vec<u16>,
    pub scales: Vec<f32>,
    pub cube: Vec<i16>,
    pub is_complex: bool,
}

pub fn decode_pointcloud2(points: &PointCloud2) -> Vec<Point> {
    let mut point_vec = Vec::with_capacity(points.height as usize * points.width as usize);
    for i in 0..points.height as usize {
        for j in 0..points.width as usize {
            let start_idx = i * points.row_step as usize + j * points.point_step as usize;
            let end_idx = start_idx + points.point_step as usize;
            let data = &points.data[start_idx..end_idx];
            let mut point = Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                noise: 0.0,
                radial_speed: 0.0,
                snr: 0.0,
                power: 0.0,
                rcs: 0.0,
            };
            for field in &points.fields {
                let offset = field.offset as usize;
                match field.datatype {
                    FLOAT32 => {
                        if field.datatype == FLOAT32 {
                            let fdata: f32 = if points.is_bigendian {
                                f32::from_be_bytes(data[offset..offset + 4].try_into().unwrap())
                            } else {
                                f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
                            };
                            match field.name.as_str() {
                                "x" => point.x = fdata,
                                "y" => point.y = fdata,
                                "z" => point.z = fdata,
                                "noise" => point.noise = fdata,
                                "radial_speed" => point.radial_speed = fdata,
                                "snr" => point.snr = fdata,
                                "power" => point.power = fdata,
                                "rcs" => point.rcs = fdata,
                                _ => {}
                            }
                        }
                    }
                    0_u8..=6_u8 | 8_u8..=u8::MAX => todo!(),
                }
            }

            point_vec.push(point);
        }
    }
    point_vec
}

pub fn open_mcap<P: AsRef<Utf8Path>>(p: P) -> Result<Mmap> {
    let fd = fs::File::open(p.as_ref()).context("Couldn't open MCAP file")?;
    unsafe { Mmap::map(&fd) }.context("Couldn't map MCAP file")
}

pub fn calculate_center(x_min: &u32, y_min: &u32, x_max: &u32, y_max: &u32) -> (u32, u32) {
    let center_x = (x_min + x_max) / 2;
    let center_y = (y_min + y_max) / 2;
    (center_x, center_y)
}

pub fn get_radar_data(points: &PointCloud2) -> Result<Vec<Point>, Error> {
    let radar_data = decode_pointcloud2(points);
    Ok(radar_data)
}

pub fn get_raw_radar_data(message: &mcap::Message<'_>) -> Result<PointCloud2, Error> {
    let points: PointCloud2 =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(points)
}

pub fn get_raw_imu_data(message: &mcap::Message<'_>) -> Result<IMU, Error> {
    let imu_data: IMU = cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(imu_data)
}

pub fn get_imu_data(imu_data: &IMU) -> Result<Imu> {
    let x = imu_data.orientation.x;
    let y = imu_data.orientation.y;
    let z = imu_data.orientation.z;
    let w = imu_data.orientation.w;

    let roll = (2.0 * (w * x + y * z)).atan2(1.0 - 2.0 * (x.powi(2) + y.powi(2)));
    let pitch = (2.0 * (w * y - z * x)).asin();
    let yaw = (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y.powi(2) + z.powi(2)));

    let roll = roll.to_degrees();
    let pitch = pitch.to_degrees();
    let yaw = yaw.to_degrees();

    let velocity_x = imu_data.angular_velocity.x;
    let velocity_y = imu_data.angular_velocity.y;
    let velocity_z = imu_data.angular_velocity.z;

    let acceleration_x = imu_data.linear_acceleration.x;
    let acceleration_y = imu_data.linear_acceleration.y;
    let acceleration_z = imu_data.linear_acceleration.z;

    Ok(Imu {
        roll,
        pitch,
        yaw,
        velocity_x,
        velocity_y,
        velocity_z,
        acceleration_x,
        acceleration_y,
        acceleration_z,
    })
}

pub fn get_raw_gps_data(message: &mcap::Message<'_>) -> Result<NavSatFix, Error> {
    let gps_data: NavSatFix =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(gps_data)
}

pub fn get_gps_data(gps_data: &NavSatFix) -> Result<Gps> {
    let latitude = gps_data.latitude;
    let longitude = gps_data.longitude;
    let altitude = gps_data.altitude;

    Ok(Gps {
        latitude,
        longitude,
        altitude,
    })
}

pub fn get_raw_obj_data(message: &mcap::Message<'_>) -> Result<ObjectsStamped, Error> {
    let deserialized_message: ObjectsStamped =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(deserialized_message)
}

pub fn get_2d_obj_data(box_2d_data: &[Object]) -> Result<Vec<Boxes2d>> {
    let mut boxes_2d = Vec::with_capacity(box_2d_data.len());
    const IMAGE_WIDTH: f32 = 1280.0;
    const IMAGE_HEIGHT: f32 = 720.0;
    for object in box_2d_data {
        if object.label == "Person" {
            let label = 1;
            let x = object.position[0];
            if x.is_nan() {
                continue;
            }
            let x_min = &object.bounding_box_2d.corners[0].kp[0];
            let y_min = &object.bounding_box_2d.corners[0].kp[1];
            let x_max = &object.bounding_box_2d.corners[2].kp[0];
            let y_max = &object.bounding_box_2d.corners[2].kp[1];
            let (center_x, center_y) = calculate_center(x_min, y_min, x_max, y_max);
            let x = center_x as f32 / IMAGE_WIDTH;
            let y = center_y as f32 / IMAGE_HEIGHT;
            let w = (x_max - x_min) as f32 / IMAGE_WIDTH;
            let h = (y_max - y_min) as f32 / IMAGE_HEIGHT;

            let distance = object.position[0];

            boxes_2d.push(Boxes2d {
                label,
                x,
                y,
                distance,
                w,
                h,
            })
        }
    }

    Ok(boxes_2d)
}

pub fn get_3d_obj_data(box_3d_data: &[Object]) -> Result<Vec<Boxes3d>> {
    let mut boxes_3d = Vec::with_capacity(box_3d_data.len());
    for object in box_3d_data {
        if object.label == "Person" {
            let label = 1;
            let x = object.position[0];
            if x.is_nan() {
                continue;
            }
            let y = object.position[1];
            let z = object.position[2];
            let w = object.dimensions_3d[0];
            let h = object.dimensions_3d[1];
            let l = object.dimensions_3d[2];
            boxes_3d.push(Boxes3d {
                label,
                x,
                y,
                z,
                w,
                h,
                l,
            })
        }
    }

    Ok(boxes_3d)
}

pub fn get_raw_image_data(message: &mcap::Message<'_>) -> Result<FoxgloveCompressedVideo, Error> {
    let image_data: FoxgloveCompressedVideo =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    if image_data.format == "h264" {
        return Ok(image_data);
    }
    Err(Error::new(ErrorKind::Other, "Video processing failed"))
}

pub fn get_raw_zed_image_data(message: &mcap::Message<'_>) -> Result<Image, Error> {
    let image_data: Image = cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(image_data)
}

pub fn get_raw_bbox_data(message: &mcap::Message<'_>) -> Result<FoxgloveImageAnnotations, Error> {
    let deserialized_message: FoxgloveImageAnnotations =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(deserialized_message)
}

pub fn get_raw_cube_data(message: &mcap::Message<'_>) -> Result<RadarCube, Error> {
    let cube_data: RadarCube =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(cube_data)
}

pub fn get_cube_data(cube_data: RadarCube) -> Result<Cube> {
    Ok(Cube {
        layout: cube_data.layout,
        shape: cube_data.shape,
        scales: cube_data.scales,
        cube: cube_data.cube,
        is_complex: cube_data.is_complex,
    })
}

pub fn get_camera_info(message: &mcap::Message<'_>) -> Result<CameraInfo, cdr::Error> {
    cdr::deserialize(&message.data)
}
