use log::error;
use log::info;
use std::fs;
use std::fs::File;
use std::io::{Write, Error};
use std::process::Command;
use std::time::Duration;

const TMP_FILE: &str = "/mnt/ram/image_latest.jpg";

#[derive(Clone, Debug)]
pub struct Camera {
    process_id: u32,
}

impl Camera {
    pub fn new() -> Self {
        Self::kill_previous_rapistill_process();
        let new_camera = Self {
            process_id: Self::start_raspistill_process(),
        };
        // wait camera process startup
        std::thread::sleep(Duration::from_secs(10));
        new_camera
    }

    /// This function waits 500ms for the picture to be taken
    pub fn take_new_pic(&self) -> Vec<u8> {
        let mut process = Command::new("kill")
            .arg("-USR1")
            .arg(format!("{}", self.process_id))
            .spawn()
            .expect("Error sending signal to camera process.");
        if !process
            .wait()
            .expect("Error waiting signal process to end.")
            .success()
        {
            error!("Process did not finish successfully.");
            panic!();
        }
        for i in 0..=9{
            // time to take picture and write to disk
            std::thread::sleep(Duration::from_millis(500));
            match fs::read(TMP_FILE){
                Ok(curr_latest) => {
                    fs::remove_file(TMP_FILE).expect(&format!("Error removing tmp file {}", TMP_FILE));
                    return curr_latest;
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
        panic!("Failed multiple times to read picture");
    }

    /// This function waits 500ms for the picture to be taken and saved
    pub fn take_new_pic_save_at(&self, path: &str) {
        let pic = self.take_new_pic();
        let mut f = File::create(path).expect(&format!("Could not create file at {}", path));
        f.write_all(&pic)
            .expect("Error writing picture to disk at new location");
    }

    fn kill_previous_rapistill_process() {
        let output = Command::new("killall")
            .arg("raspistill")
            .output()
            .expect("Could not kill previous raspistill process");
        if output.status.success() {
            info!("Killed previous raspistill process!");
        } else {
            info!("No previous raspistill process detected!");
        }
    }

    fn start_raspistill_process() -> u32 {
        // sudo mount -t tmpfs -o rw,size=50M tmpfs /mnt/ramdisk
        info!("Starting raspistill process");
        let process = Command::new("raspistill")
            .arg("-q") // quality 7
            .arg("7")
            .arg("-w")
            .arg("1640")
            .arg("-h")
            .arg("1232")
            .arg("-s") // signal mode
            .arg("-n") // no preview window
            .arg("-ex") // sports exposure
            .arg("sports")
            // .arg("-a") // a
            // .arg("4")
            .arg("-a") // annotate day/month/year hour
            .arg("8") // annotate day/month/year hour
            .arg("-a") // annotate day/month/year hour
            // .arg("%d")
            .arg("%d-%m-%Y %X")
            .arg("-o") // output to
            .arg(TMP_FILE)
            .spawn()
            .expect("raspistill process failed to start");
        let camera_process_id = process.id();
        info!("Process started with id: {}", camera_process_id);
        camera_process_id
    }
}
