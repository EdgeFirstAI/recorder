use anyhow::{Context, Result};
use camino::Utf8Path;
use memmap::Mmap;
use serde::{Deserialize, Serialize};
use std::result::Result::Ok;
use std::{fs, io::Cursor};

use anyhow::Error;

use mcap::MessageStream;
use zenoh_ros_type::sensor_msgs::{point_field::FLOAT32, PointCloud2, IMU};

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

fn read_mcap(mapped: Mmap) -> Result<()> {
    for message_result in MessageStream::new(&mapped)? {
        let message: mcap::Message<'_> = message_result?;

        if message.channel.topic == "/smart_radar/targets_0" {
            match get_raw_radar_data(message.clone()) {
                Ok(raw_radar_data) => match get_radar_data(raw_radar_data) {
                    Ok(radar_data) => {
                        println!("{:?}", radar_data.len());
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
    }
    Ok(())
}
fn main() -> Result<()> {
    let mapped: Mmap = open_mcap("test.mcap")?;
    let _ = read_mcap(mapped);

    Ok(())
}
