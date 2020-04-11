//raspistill -st -q 7 -w 1640 -h 1232 -t 300000 -tl 700 -n -ex sports -o image7_%04d.jpg
//ffmpeg -framerate 10 -i image7_%04d.jpg -video_size 1640:1232 -c:v h264_omx -bufsize 64k -b:v 1.2M -vf fps=10 out.mp4
#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]
#[macro_use]
extern crate rocket;

mod camera_api;

use log::error;
use rocket::State;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

mod timelapse;
use rocket_seek_stream::SeekStream;

// #[get("/stream")]
// fn stream<'a>() -> std::io::Result<SeekStream<'a>> {
//     SeekStream::from_path("Sintel.mp4")
// }

fn main() {
    use flexi_logger::colored_opt_format;
    flexi_logger::Logger::with_str("info")
        .format(colored_opt_format)
        .log_to_file()
        .directory("./logs")
        .start()
        .unwrap();
    // let camera = camera_api::Camera::new();
    // let pic = camera.take_new_pic();
    let mut k = timelapse::TimeLapseManufacturer::new();
    k.run();
    // rocket::ignite().mount("/", routes![stream]).launch();
}
