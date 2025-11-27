// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

pub fn read_mcap(mapped: Mmap) -> Result<()> {
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
                        println!("GPS {:?}", gps_data);
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

        if message.channel.topic == "/camera/compressed" {
            match get_raw_image_data(message.clone()) {
                Ok(raw_img_data) => {
                    println!("{:?}", raw_img_data.data)
                }
                Err(err) => {
                    eprintln!("Error while getting raw obj data: {}", err);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mcap_open() {
        let file_path = "test.mcap";
        let mapped = match open_mcap(file_path) {
            Ok(mapped) => mapped,
            Err(_) => {
                panic!("Failed to open MCAP file");
            }
        };
    }
    fn test_read_mcap() {
        match read_mcap(mapped) {
            Ok(_) => {
                read_mcap(mapped);
            }
            Err(_) => {
                panic!("Failed to read from the MCAP file");
            }
        }
    }
}
