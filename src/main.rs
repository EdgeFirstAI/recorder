use anyhow::{Context, Result};
use camino::Utf8Path;
use memmap::Mmap;
use serde::{Deserialize, Serialize};
use std::result::Result::Ok;
use std::{fs, io::Cursor};
use zenoh_ros_type::std_msgs::Header;

use anyhow::Error;

use serde_big_array::BigArray;

use mcap::MessageStream;
use zenoh_ros_type::sensor_msgs::{point_field::FLOAT32, Image, NavSatFix, PointCloud2, IMU};

#[derive(Debug, Serialize, Deserialize)]
struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub noise: f32,
    pub radial_speed: f32,
    pub snr: f32,
    pub power: f32,
    pub rcs: f32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Imu {
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
#[derive(Debug, Serialize, Deserialize)]
struct Gps {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Boxes3d {
    pub label: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
    pub l: f32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Boxes2d {
    pub label: i32,
    pub x: f32,
    pub y: f32,
    pub distance: f32,
    pub w: f32,
    pub h: f32,
}
#[derive(Debug, Deserialize, Serialize)]
struct Object {
    label: String,
    label_id: i16,
    sublabel: String,
    confidence: f32,
    position: [f32; 3],
    position_covariance: [f32; 6],
    velocity: [f32; 3],
    tracking_available: bool,
    tracking_state: i8,
    action_state: i8,
    bounding_box_2d: BoundingBox2Di,
    bounding_box_3d: BoundingBox3D,
    dimensions_3d: [f32; 3],
    skeleton_available: bool,
    body_format: i8,
    head_bounding_box_2d: BoundingBox2Df,
    head_bounding_box_3d: BoundingBox3D,
    head_position: [f32; 3],
    skeleton_2d: Skeleton2D,
    skeleton_3d: Skeleton3D,
}
#[derive(Debug, Deserialize, Serialize)]
struct BoundingBox2Df {
    corners: [Keypoint2Df; 4],
}

#[derive(Debug, Deserialize, Serialize)]
struct BoundingBox2Di {
    corners: [Keypoint2Di; 4],
}

#[derive(Debug, Deserialize, Serialize)]
struct BoundingBox3D {
    corners: [Keypoint3D; 8],
}

#[derive(Debug, Deserialize, Serialize)]
struct Keypoint2Df {
    kp: [f32; 2],
}

#[derive(Debug, Deserialize, Serialize)]
struct Keypoint2Di {
    kp: [u32; 2],
}

#[derive(Debug, Deserialize, Serialize)]
struct Keypoint3D {
    kp: [f32; 3],
}

#[derive(Debug, Deserialize, Serialize)]
struct Skeleton2D {
    #[serde(with = "BigArray")]
    kp: [Keypoint2Df; 70],
}

#[derive(Debug, Deserialize, Serialize)]
struct Skeleton3D {
    #[serde(with = "BigArray")]
    kp: [Keypoint3D; 70],
}

#[derive(Deserialize, Serialize)]
struct ObjectsStamped {
    header: Header,
    objects: Vec<Object>,
}

fn decode_pointcloud2(points: &PointCloud2) -> Vec<Point> {
    let mut point_vec = Vec::new();
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
                        let fdata: f32;
                        if points.is_bigendian {
                            fdata =
                                f32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
                        } else {
                            fdata =
                                f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                        }
                        match field.name.as_str() {
                            "x" => {
                                point.x = fdata;
                            }
                            "y" => {
                                point.y = fdata;
                            }
                            "z" => {
                                point.z = fdata;
                            }
                            "noise" => {
                                point.noise = fdata;
                            }
                            "radial_speed" => {
                                point.radial_speed = fdata;
                            }
                            "snr" => {
                                point.snr = fdata;
                            }
                            "power" => {
                                point.power = fdata;
                            }
                            "rcs" => {
                                point.rcs = fdata;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            point_vec.push(point);
        }
    }
    return point_vec;
}

fn open_mcap<P: AsRef<Utf8Path>>(p: P) -> Result<Mmap> {
    let fd = fs::File::open(p.as_ref()).context("Couldn't open MCAP file")?;
    unsafe { Mmap::map(&fd) }.context("Couldn't map MCAP file")
}

fn calculate_center(x_min: &u32, y_min: &u32, x_max: &u32, y_max: &u32) -> (u32, u32) {
    let center_x = (x_min + x_max) / 2;
    let center_y = (y_min + y_max) / 2;
    (center_x, center_y)
}

fn get_radar_data(points: PointCloud2) -> Result<Vec<Point>, Error> {
    let radar_data = decode_pointcloud2(&points);
    Ok(radar_data)
}

fn get_raw_radar_data(message: mcap::Message<'_>) -> Result<PointCloud2, Error> {
    let cursor = Cursor::new(message.data.into_owned());
    let points = cdr::deserialize_from::<_, PointCloud2, _>(cursor, cdr::Infinite)?;
    Ok(points)
}

fn get_raw_imu_data(message: mcap::Message<'_>) -> Result<IMU, Error> {
    let cursor = Cursor::new(message.data.into_owned());
    let imu_data = cdr::deserialize_from::<_, IMU, _>(cursor, cdr::Infinite)?;
    Ok(imu_data)
}

fn get_imu_data(imu_data: IMU) -> Result<Imu> {
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

fn get_raw_gps_data(message: mcap::Message<'_>) -> Result<NavSatFix, Error> {
    let cursor = Cursor::new(message.data.into_owned());
    let gps_data = cdr::deserialize_from::<_, NavSatFix, _>(cursor, cdr::Infinite)?;
    Ok(gps_data)
}

fn get_gps_data(gps_data: NavSatFix) -> Result<Gps> {
    let latitude = gps_data.latitude;
    let longitude = gps_data.longitude;
    let altitude = gps_data.altitude;

    Ok(Gps {
        latitude,
        longitude,
        altitude,
    })
}

fn get_raw_obj_data(message: mcap::Message<'_>) -> Result<ObjectsStamped, Error> {
    let deserialized_message: ObjectsStamped =
        cdr::deserialize(&message.data).expect("Failed to deserialize message");
    Ok(deserialized_message)
}

fn get_2d_obj_data(box_2d_data: Vec<Object>) -> Result<Vec<Boxes2d>> {
    let mut boxes_2d = Vec::new();
    const IMAGE_WIDTH: f32 = 1280.0;
    const IMAGE_HEIGHT: f32 = 720.0;
    for (_index, object) in box_2d_data.iter().enumerate() {
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

fn get_3d_obj_data(box_3d_data: Vec<Object>) -> Result<Vec<Boxes3d>> {
    let mut boxes_3d = Vec::new();
    for (_index, object) in box_3d_data.iter().enumerate() {
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

fn get_raw_image_data(message: mcap::Message<'_>) -> Result<Image, Error> {
    let cursor = Cursor::new(message.data.into_owned());
    let image_data = cdr::deserialize_from::<_, Image, _>(cursor, cdr::Infinite)?;
    Ok(image_data)
}

fn read_mcap(mapped: Mmap) -> Result<()> {
    for message_result in MessageStream::new(&mapped)? {
        let message: mcap::Message<'_> = message_result?;

        if message.channel.topic == "/smart_radar/targets_0" {
            match get_raw_radar_data(message.clone()) {
                Ok(raw_radar_data) => match get_radar_data(raw_radar_data) {
                    Ok(radar_data) => {
                        println!("{:?}", radar_data);
                    }
                    Err(err) => {
                        eprintln!("Error while getting radar data: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("Error while getting radar data: {}", err);
                }
            }
        }

        if message.channel.topic == "/imu" {
            match get_raw_imu_data(message.clone()) {
                Ok(raw_imu_data) => match get_imu_data(raw_imu_data) {
                    Ok(imu_data) => {
                        println!("{:?}", imu_data);
                    }
                    Err(err) => {
                        eprintln!("Error while getting imu data: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("Error while getting raw imu data: {}", err);
                }
            }
        }

        if message.channel.topic == "/gps" {
            match get_raw_gps_data(message.clone()) {
                Ok(raw_gps_data) => match get_gps_data(raw_gps_data) {
                    Ok(gps_data) => {
                        println!("{:?}", gps_data);
                    }
                    Err(err) => {
                        eprintln!("Error while getting gps data: {}", err);
                    }
                },

                Err(err) => {
                    eprintln!("Error while getting raw gps data: {}", err);
                }
            }
        }

        if message.channel.topic == "/zed/zed_node/obj_det/objects" {
            match get_raw_obj_data(message.clone()) {
                Ok(raw_obj_data) => match get_2d_obj_data(raw_obj_data.objects) {
                    Ok(obj_data) => {
                        println!("{:?}", obj_data);
                    }
                    Err(err) => {
                        eprintln!("Error while getting obj data: {}", err);
                    }
                },

                Err(err) => {
                    eprintln!("Error while getting raw obj data: {}", err);
                }
            }
        }

        if message.channel.topic == "/zed/zed_node/left_raw/image_raw_color" {
            match get_raw_image_data(message.clone()) {
                Ok(raw_img_data) => {
                    println!("{:?}", raw_img_data.encoding)
                }
                Err(err) => {
                    eprintln!("Error while getting raw obj data: {}", err);
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let mapped: Mmap = open_mcap("test.mcap")?;
    let _ = read_mcap(mapped);

    Ok(())
}
