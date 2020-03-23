//raspistill -st -q 7 -w 1640 -h 1232 -t 300000 -tl 700 -n -ex sports -o image7_%04d.jpg


//ffmpeg -framerate 10 -i image7_%04d.jpg -video_size 1640:1232 -c:v h264_omx -bufsize 64k -b:v 1.2M -vf fps=10 out.mp4


#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::io::Read;
use std::fs;
use rocket::State;

const LATEST_IMG_NAME: &str = "/mnt/ramdisk/image_latest.jpg";

#[get("/get_latest_img")]
fn get_lastest_img(camera_id: State<u32>) -> Vec<u8> {
    let mut process = Command::new("kill")
        .arg("-USR1")
        .arg(format!("{}", *camera_id))
        .spawn().unwrap();
    if !process.wait().unwrap().success(){
        println!("process did not finish successfully");
    }

    let curr_latest = fs::read(LATEST_IMG_NAME).unwrap();
//    fs::remove_file(LATEST_IMG_NAME).unwrap();
    curr_latest
}

fn main() {
    Command::new("killall")
        .arg("raspistill")
        .output().expect("Could not kill previous raspistill process");

    // raspistill -vf -hf -roi 0,0.15,0.95,0.55 -th none -n -s -q 10 -t 0 -o gringos_now.jpg
    let mut process = Command::new("raspistill")
        .arg("-q")
        .arg("7")
        .arg("-w")
        .arg("1640")
        .arg("-h")
        .arg("1232")
        .arg("-s")
        .arg("-n")
        .arg("-ex")
        .arg("sports")
        .arg("-o")
        .arg(LATEST_IMG_NAME.clone())
        .spawn()
        .expect("command failed to start");
    let camera_process_id = process.id();
    if let Some(out) = &mut process.stderr{
        let mut string = String::new();
        out.read_to_string(&mut string).unwrap();
        println!("{}", string);
        panic!();
    }

    if let Some(out) = &mut process.stdout{
        let mut string = String::new();
        out.read_to_string(&mut string).unwrap();
        println!("{}", string);
    }
    rocket::ignite().manage(camera_process_id).
        mount("/", routes![get_lastest_img]).launch();


//    std::thread::sleep(Duration::from_millis(1500));
//    for i in 0..=10{
//        let process = Command::new("kill")
//            .arg("-USR1")
//            .arg(format!("{}", camera_process_id))
//            .spawn().unwrap();
//        std::thread::sleep(Duration::from_millis(1000));
//        curr_latest = fs::read(LATEST_IMG_NAME).unwrap();
//        println!("Size = {}", curr_latest.len());
//        std::fs::rename(LATEST_IMG_NAME, format!("some_{}.jpg", i)).unwrap();
//    }
//
//    if let Err(error) = ws::listen("127.0.0.1:3012", |out| {
//
//        // The handler needs to take ownership of out, so we use move
//        out.send("Some msg!").unwrap();
//        move |msg| {
//
//            // Handle messages received on this connection
//            println!("Server got message '{}'. ", msg);
//            Ok(())
//        }
//
//    }) {
//        // Inform the user of failure
//        println!("Failed to create WebSocket due to {:?}", error);
//    }
//    process.kill().unwrap();
}
