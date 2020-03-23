//raspistill -st -q 7 -w 1640 -h 1232 -t 300000 -tl 700 -n -ex sports -o image7_%04d.jpg


//ffmpeg -framerate 10 -i image7_%04d.jpg -video_size 1640:1232 -c:v h264_omx -bufsize 64k -b:v 1.2M -vf fps=10 out.mp4


//ffmpeg -framerate 10 -i image%04d.jpg -video_size 1640:1232 -c:v h264_omx -b:v 1.2M -vf fps=10 out.mp4


use std::process::{Command, Child};
use std::thread::sleep;
use std::time::{Duration, Instant, UNIX_EPOCH, SystemTime};
use std::io::{Write};
use std::fs::{File, remove_file};
use reqwest::Client;

const IMAGE_DIR: &str = "/mnt/SkynetStorage/imgs";
const VIDEO_DIR: &str = "/mnt/SkynetStorage/videos";
async fn get_1000_imgs(client: &Client, suffix: &str){
    for i in 0..=10{
        let start = Instant::now();
        let img = client.get("htpp://192.168.15.26:8000/get_latest_img")
            .send()
            .await.unwrap()
            .bytes()
            .await.unwrap();

        let mut f = File::create(format!("{}/image{:04}{}.jpg", IMAGE_DIR, i, suffix)).unwrap();
        f.write_all(&img).unwrap();
        let elasped_millis = start.elapsed().as_millis();
        if elasped_millis < 500{
            sleep(Duration::from_millis(500 - elasped_millis as u64));
        }
        if elasped_millis > 1000{
            println!("Took {} to get img", elasped_millis);
        }
    }
}

fn encode_imgs_to_video(img_suffix: &str, video_suffix: &str) -> Child{
    let process = Command::new("ffmpeg")
        //-i image%04d.jpg -video_size 1640:1232 -c:v h264_omx -b:v 1.2M -vf fps=10 out.mp4
        .arg("-framerate")
        .arg("10")
        .arg("-i")
        .arg(format!("{}/image%04d{}.jpg",IMAGE_DIR, img_suffix))
        .arg("-video_size")
        .arg("1640:1232")
//            .arg("-c:v")
//            .arg("h264_omx")
//        .arg("-b:v")
//        .arg("2.5M")
        .arg("-vf")
        .arg("fps=10")
        .arg(format!("{}/{}.mp4", VIDEO_DIR, video_suffix))
        .spawn()
        .expect("command failed to start");
    process
}

pub fn remove_file_over_max(){
    use walkdir::WalkDir;
    let mut files: Vec<MovieFile> = vec![];
    for entry in WalkDir::new(VIDEO_DIR) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".mp4"){

            files.push(MovieFile{
                path: entry.path().to_string_lossy().to_string(),
                filename_no_extension: entry
                    .file_name()
                    .to_string_lossy()
                    .to_string()
                    .replace(".mp4", "")
                    .parse::<u64>().unwrap()
            });
        }
    }
    if files.len() >= 20{
        let mut oldest = files.get(0).unwrap();

        for file in &files{
            if file.filename_no_extension < oldest.filename_no_extension{
                oldest = &file;
            }
        }
        remove_file(&oldest.path).unwrap();
        println!("removed {}", oldest.path);
    }
}

struct MovieFile{
    path: String,
    filename_no_extension: u64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let client: Client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    let mut child_handle: Option<Child> = None;

    let mut a_time = true;
    loop{
        remove_file_over_max();
        let img_suffix = if a_time{
            "_a"
        }else{
            "_b"
        };
        a_time = !a_time;
        get_1000_imgs(&client, img_suffix).await;
        // check if last encoding finished
        if let Some(mut child_handle) = child_handle{
            let status = child_handle.try_wait().expect("Encoder thread is not finished!!").unwrap();
            if !status.success(){
                panic!("Encoder errored");
            }
        }
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        child_handle = Some(encode_imgs_to_video(img_suffix, &format!("{}", timestamp)));
    }

//    Ok(())

}
