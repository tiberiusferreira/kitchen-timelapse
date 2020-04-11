use super::camera_api;
use crate::camera_api::Camera;
use chrono::prelude::*;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use std::fs;
use std::thread::JoinHandle;
use std::time::Duration;
use std::io::Write;
use std::process::Command;

mod encoder;

const PICS_FOLDER_ROOT: &str = "/mnt/skynet/pics";
const MOVIES_FOLDER_ROOT: &str = "/mnt/skynet/movies";
const ENCODING_FOLDER: &str = "/mnt/skynet/encoding";

pub enum PicTakingMessage {
    Done,
}

struct EncodingOutput{
    output_path: String,
    filename: String,
}
pub enum EncodingMessage {
    Done(EncodingOutput),
}
pub struct TimeLapseManufacturer {
    camera: Camera,
    curr_tmp_pic_recording_folder: PicsFolders,
    picture_taking_thread: Option<Receiver<PicTakingMessage>>,
    encoding_thread: Option<Receiver<EncodingMessage>>,
}

#[derive(Clone, Debug, PartialEq)]
enum PicsFolders {
    A,
    B,
}

impl PicsFolders {
    pub fn path(&self) -> String {
        format!("{}/{}", PICS_FOLDER_ROOT, self.to_string())
    }
    pub fn delete_folder(&self) {
        std::fs::remove_dir_all(self.path())
            .expect(&format!("Could not clear dir: {}", self.path()));
    }

    pub fn reset_folder(&self) {
        self.delete_folder();
        self.create_folder();
    }

    pub fn create_folder(&self) {
        fs::create_dir_all(self.path()).expect(&format!(
            "Error creating pic folder in path: {}",
            self.path()
        ));
    }

    pub fn switch_folders(&mut self){
        match self{
            PicsFolders::A => {
                *self = PicsFolders::B;
            },
            PicsFolders::B => {
                *self = PicsFolders::A;
            },
        };
    }
}
impl ToString for PicsFolders {
    fn to_string(&self) -> String {
        match &self {
            PicsFolders::A => "a".to_string(),
            PicsFolders::B => "b".to_string(),
        }
    }
}

#[derive(Debug)]
struct Movie {
    filename: String,
    timestamp: i64,
    datetime: NaiveDateTime,
    path: String,
}

#[derive(Debug)]
pub struct TodayFolder {
    timestamp: i64,
    datetime: NaiveDateTime,
    today_movies: Vec<Movie>,
    path: String,
}
#[derive(Debug)]
pub struct DirStructure {
    movies: Vec<Movie>,
    today_folder: Option<TodayFolder>,
}

impl TimeLapseManufacturer {
    pub fn get_dir_structure() -> DirStructure {
        let mut dir_structure: DirStructure = DirStructure {
            movies: vec![],
            today_folder: None,
        };
        if fs::read_dir(MOVIES_FOLDER_ROOT).is_err(){
            fs::create_dir_all(MOVIES_FOLDER_ROOT).unwrap();
        }
        let dir = fs::read_dir(MOVIES_FOLDER_ROOT).expect("Error movies folder dir");
        for entry in dir {
            let entry = entry.expect("Error reading entry from movies folder dir");
            let metadata = entry.metadata().expect("No metadata for entry!");
            let is_file = metadata.file_type().is_file();
            let is_mp4 = entry.file_name().to_string_lossy().ends_with(".mp4");
            if is_file && is_mp4 {
                let timestamp = entry.file_name().to_string_lossy().replace(".mp4", "");
                let timestamp: i64 = timestamp
                    .parse()
                    .expect(&format!("Filename was not timestamp: {}", timestamp));
                let datetime = chrono::NaiveDateTime::from_timestamp(timestamp, 0);
                dir_structure.movies.push(Movie {
                    filename: entry.file_name().to_string_lossy().to_string(),
                    timestamp,
                    datetime,
                    path: entry.path().to_string_lossy().to_string(),
                })
            } else if metadata.is_dir() {
                let filename = entry.file_name();
                let timestamp = filename.to_string_lossy();
                if let Ok(timestamp) = timestamp.parse::<i64>() {
                    let datetime = chrono::NaiveDateTime::from_timestamp(timestamp, 0);
                    assert!(dir_structure.today_folder.is_none(), "two today folders!");
                    dir_structure.today_folder = Some(TodayFolder {
                        timestamp,
                        datetime,
                        today_movies: vec![],
                        path: entry.path().to_string_lossy().to_string(),
                    });
                    let entries =
                        fs::read_dir(entry.path()).expect("Error reading today_movies folder");
                    for entry in entries {
                        let entry = entry.expect("Error reading entry in today_movies folder");
                        let is_mp4 = entry.file_name().to_string_lossy().ends_with(".mp4");
                        let is_file = entry.metadata().unwrap().file_type().is_file();
                        if is_file && is_mp4 {
                            let timestamp = entry.file_name().to_string_lossy().replace(".mp4", "");
                            let timestamp: i64 = timestamp
                                .parse()
                                .expect(&format!("Filename was not timestamp: {}", timestamp));
                            let datetime = chrono::NaiveDateTime::from_timestamp(timestamp, 0);
                            dir_structure
                                .today_folder
                                .as_mut()
                                .unwrap()
                                .today_movies
                                .push(Movie {
                                    filename: entry.file_name().to_string_lossy().to_string(),
                                    timestamp,
                                    datetime,
                                    path: entry.path().to_string_lossy().to_string(),
                                })
                        }
                    }
                }
            }
        }
        dir_structure
    }
    pub fn new() -> Self {
        Self::clear_pics_folder();
        Self {
            camera: Camera::new(),
            curr_tmp_pic_recording_folder: PicsFolders::A,
            picture_taking_thread: None,
            encoding_thread: None
        }
    }

    fn clear_pics_folder() {
        use std::fs;
        if fs::read_dir(PICS_FOLDER_ROOT).is_ok() {
            fs::remove_dir_all(PICS_FOLDER_ROOT).expect("Error removing PICS folder");
        }
        PicsFolders::A.create_folder();
        PicsFolders::B.create_folder();
    }


    fn maybe_stitch_today_videos(&mut self){
        // check if "today" has ended and we have to stitch it into a single video
        let dir_structure = Self::get_dir_structure();
        if let Some(today_folder)  = dir_structure.today_folder{
            if today_folder.datetime.num_days_from_ce() != chrono::Utc::now().naive_utc().num_days_from_ce(){
                // today has ended
                unimplemented!()
            }
        }
    }

    pub fn run(&mut self) {

        println!("{:?}", Self::get_dir_structure());


        self.curr_tmp_pic_recording_folder.reset_folder();
        self.start_take_pictures_till_hour_end_thread();

        // wait for pic taking thread to be done
        let _message = self.picture_taking_thread.as_mut().unwrap().recv().unwrap();
        self.picture_taking_thread = None;

        // start encoding the pics just taken
        let filename = format!("{}.mp4", chrono::Utc::now().timestamp());
        let tmp_output_dir = format!("{}/{}", ENCODING_FOLDER, "today");
        if fs::read_dir(&tmp_output_dir).is_err(){
            fs::create_dir_all(tmp_output_dir).unwrap();
        }
        self.start_encoding_thread(
            format!("{}", self.curr_tmp_pic_recording_folder.path()),
            format!(
                "{}/today/{}",
                ENCODING_FOLDER,
                filename
            ),
            filename.to_string()
        );

        // switch pics folder
        self.curr_tmp_pic_recording_folder.switch_folders();

        // wait encoding to be over and get output path
        let encoding_output = match self.encoding_thread.as_mut().unwrap().recv().unwrap(){
            EncodingMessage::Done(output_path) => {
                output_path
            },
        };

        // check if we already a "today" folder, if not create one
        if Self::get_dir_structure().today_folder.is_none(){
            let today_folder_path = format!("{}/{}", MOVIES_FOLDER_ROOT, chrono::Utc::now().timestamp());
            fs::create_dir_all(&today_folder_path).expect(&format!("Error creating {}", today_folder_path));
        }
        let today_folder = Self::get_dir_structure().today_folder;
        assert!(today_folder.is_some(), "Error creating today folder");
        let today_folder = today_folder.unwrap();
        let dest = format!("{}/{}", today_folder.path, encoding_output.filename);

        // move the encoded movie into today folder
        fs::rename(&encoding_output.output_path, &dest)
            .expect(&format!("Error moving {} to {}", encoding_output.output_path, dest));

        self.stitch();
        // check if we need to stitch


    }

    fn stitch(&mut self){
        let mut files_string = String::new();
        let structure = Self::get_dir_structure();
        if let Some(folder) = structure.today_folder {

            for movie in folder.today_movies {
                files_string.push_str(&format!("file \'{}\'\n", movie.path));
            }

            let mut file = fs::File::create("files.txt").unwrap();
            file.write_all(files_string.as_bytes()).unwrap();
            // ffmpeg -f concat -safe 0 -i files.txt -c copy some.mp4
            let out_path = format!("{}/{}.mp4", ENCODING_FOLDER, folder.timestamp);
            let mut process = Command::new("ffmpeg")
                .arg("-f")
                .arg("concat")
                .arg("-safe")
                .arg("0")
                .arg("-i")
                .arg("files.txt")
                .arg("-c")
                .arg("copy")
                .arg(&out_path)
                .spawn()
                .expect("command failed to start");
            process.wait().unwrap();
            fs::remove_dir_all(&folder.path).expect("Error removing today folder");
            fs::rename(&out_path, format!("{}.mp4", &folder.path)).expect(&format!("Error moving {} to {}",out_path, folder.path));
        }

    }

    fn start_take_pictures_till_hour_end_thread(&mut self) -> JoinHandle<()> {
        let camera_process = self.camera.clone();
        let recording_folder = self.curr_tmp_pic_recording_folder.clone();
        let (sender, receiver) = crossbeam_channel::bounded::<PicTakingMessage>(2);
        self.picture_taking_thread = Some(receiver);
        std::thread::spawn(move || {
            let local: DateTime<Local> = Local::now();
            let initial_hour = local.hour();
            let mut i = 0;
            while Local::now().hour() == initial_hour {
                let path = format!(
                    "{}/{}/{:05}.jpg",
                    PICS_FOLDER_ROOT,
                    recording_folder.to_string(),
                    i
                );
                camera_process.take_new_pic_save_at(&path);
                i += 1;
                std::thread::sleep(Duration::from_secs_f32(0.5));

                if i == 20 {
                    break;
                }
            }
            sender.send(PicTakingMessage::Done).unwrap();
        })
    }
}
