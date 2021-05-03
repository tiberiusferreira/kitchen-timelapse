use crate::camera_api::Camera;
use chrono::prelude::*;
use crossbeam_channel::Receiver;
use log::{error, info};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::ops::Sub;
use chrono::Duration;

mod encoder;

const PICS_FOLDER_ROOT: &str = "/mnt/skynet/pics";
const MOVIES_FOLDER_ROOT: &str = "/mnt/skynet/movies";
const ENCODING_FOLDER: &str = "/mnt/skynet/encoding";

pub enum PicTakingMessage {
    Done,
}

pub struct EncodingOutput {
    output_path_with_filename: String,
    filename: String,
}
pub enum EncodingMessage {
    Done(EncodingOutput),
}
pub struct TimeLapseManufacturer {
    camera: Camera,
    curr_tmp_pic_recording_folder: PicsFolders,
    picture_taking_thread: Option<(chrono::DateTime<Local>, Receiver<PicTakingMessage>)>,
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
        info!("Deleting folder: {}", self.path());
        if std::fs::read_dir(self.path()).is_ok(){
            std::fs::remove_dir_all(self.path())
                .expect(&format!("Could not clear dir: {}", self.path()));
        }
    }

    pub fn reset_folder(&self) {
        self.delete_folder();
        self.create_folder();
    }

    pub fn get_other_one(&self) -> PicsFolders {
        return match &self {
            PicsFolders::A => PicsFolders::B,
            PicsFolders::B => PicsFolders::A,
        };
    }

    pub fn create_folder(&self) {
        info!("Creating dir: {}", self.path());
        fs::create_dir_all(self.path()).expect(&format!(
            "Error creating pic folder in path: {}",
            self.path()
        ));
    }

    pub fn switch_folders(&mut self) {
        match self {
            PicsFolders::A => {
                *self = PicsFolders::B;
            }
            PicsFolders::B => {
                *self = PicsFolders::A;
            }
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
    /// this should be a number which is the timestamp of when this movie started recording
    /// with the .mp4 extension
    filename: String,
    /// this should be a number which is the timestamp of when this movie started recording
    timestamp: i64,
    datetime: NaiveDateTime,
    path: String,
}

#[derive(Debug)]
pub struct TodayFolder {
    /// this timestamp stores which which day this folder is from
    timestamp: i64,
    /// this datetime stores which which day this folder is from
    datetime: NaiveDateTime,
    today_movies: Vec<Movie>,
    path: String,
}

#[derive(Debug)]
pub struct DirStructure {
    /// Movies of the last days, the scope of a single movie is a whole day of the week
    movies: Vec<Movie>,
    /// folder storing short movies, each movie scope is a single hour of the day
    today_folder: Option<TodayFolder>,
}

impl TimeLapseManufacturer {
    /// All the files in MOVIES_FOLDER_ROOT should either be a folder (maximum of one folder, the today folder)
    /// with its name being a timestamp or a file, with its name being a timestamp and extension .mp4
    /// The folder should also contain files with extension .mp4 and named a timestamp number.
    pub fn get_dir_structure() -> DirStructure {
        let mut dir_structure: DirStructure = DirStructure {
            movies: vec![],
            today_folder: None,
        };
        if fs::read_dir(MOVIES_FOLDER_ROOT).is_err() {
            info!("Movies root folder not found, creating one.");
            fs::create_dir_all(MOVIES_FOLDER_ROOT).expect("Error creating movies folder root dir");
        }
        let dir = fs::read_dir(MOVIES_FOLDER_ROOT).expect("Error creating movies folder dir");
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
                    assert!(
                        dir_structure.today_folder.is_none(),
                        "Two today folders found!"
                    );
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
        info!("Clearing pics folder");
        Self::clear_pics_folder();
        Self {
            camera: Camera::new(),
            curr_tmp_pic_recording_folder: PicsFolders::A,
            picture_taking_thread: None,
            encoding_thread: None,
        }
    }

    fn clear_pics_folder() {
        if fs::read_dir(PICS_FOLDER_ROOT).is_ok() {
            info!("Found previous pic dir. Removing dir: {}", PICS_FOLDER_ROOT);
            fs::remove_dir_all(PICS_FOLDER_ROOT).expect("Error removing PICS folder");
        }
        PicsFolders::A.create_folder();
        PicsFolders::B.create_folder();
    }

    pub fn encode_last_hour_and_move_to_today_folder_clean_pic_folder(&mut self) {
        // start encoding the pics just taken
        let last_hour_timestamp = Local::now().sub(Duration::minutes(30)).timestamp();
        // get the folder not currently active, the one which just finished being filled with photos
        let pics_folder = self.curr_tmp_pic_recording_folder.get_other_one();
        let encoded_movie_filename = format!("{}.mp4", last_hour_timestamp);
        let tmp_output_dir = format!("{}/{}", ENCODING_FOLDER, "today");
        if fs::read_dir(&tmp_output_dir).is_err() {
            info!("Creating new today folder at: {}", &tmp_output_dir);
            fs::create_dir_all(&tmp_output_dir).expect(&format!(
                "Error creating dir for encoding: {}",
                tmp_output_dir
            ));
        }

        self.start_encoding_thread(
            format!(
                "{}",
                pics_folder.path()
            ),
            format!("{}/{}", tmp_output_dir, encoded_movie_filename),
            encoded_movie_filename.to_string(),
        );

        // wait encoding to be over and get output path
        info!("Waiting for encoding thread...");
        let encoding_output = match self
            .encoding_thread
            .as_mut()
            .expect("Encoding thread panicked!")
            .recv()
            .expect("Encoding thread did not send done MSG!")
        {
            EncodingMessage::Done(output_path) => output_path,
        };
        info!("Enconding of last hour done! Deleting pics from folder.");
        pics_folder.reset_folder();
        // check if we already a "today" folder, if not create one
        if Self::get_dir_structure().today_folder.is_none() {
            let today_folder_path = format!(
                "{}/{}",
                MOVIES_FOLDER_ROOT,
                chrono::Local::now().timestamp()
            );
            info!("No today folder yet, creating one at {}", today_folder_path);
            fs::create_dir_all(&today_folder_path)
                .expect(&format!("Error creating {}", today_folder_path));
        }

        let today_folder = Self::get_dir_structure().today_folder;
        let today_folder = today_folder.expect("Error creating today folder");
        let dest_path_with_filename = format!("{}/{}", today_folder.path, encoding_output.filename);

        // move the encoded movie into today folder
        info!(
            "Moving the encoded file from {} to {}",
            encoding_output.output_path_with_filename, dest_path_with_filename
        );
        fs::rename(
            &encoding_output.output_path_with_filename,
            &dest_path_with_filename,
        )
        .expect(&format!(
            "Error moving {} to {}",
            encoding_output.output_path_with_filename, dest_path_with_filename
        ));
    }

    pub fn start_taking_pictures(&mut self) {
        self.curr_tmp_pic_recording_folder.reset_folder();
        self.start_take_pictures_till_hour_end_thread();
    }

    pub fn wait_taking_pictures(&mut self) {
        // wait for pic taking thread to be done
        let _message = self
            .picture_taking_thread
            .as_mut()
            .expect("There was no pic taking thread receiver after taking pics")
            .1
            .recv()
            .expect("Pic taking thread did not send any msg");
        self.picture_taking_thread = None;
    }

    /// Start pic taking at current folder
    /// Wait Pic taking done
    /// When done switch folder and restart
    /// start encoding to TMP folder, wait for it, move from TMP to today folder
    /// check need stitching, if so do it and wait for it
    /// wait pic taking done, switch folder
    pub fn run(&mut self) {
        // Starting Pic taking
        println!("{:#?}", Self::get_dir_structure());
        if fs::read_dir(ENCODING_FOLDER).is_ok(){
            info!("Encoding folder found! Removing previous encoding folder");
            fs::remove_dir_all(ENCODING_FOLDER).expect("Error removing previous enconding folder!");
        }
        PicsFolders::A.delete_folder();
        PicsFolders::B.delete_folder();
        loop {
            let start_pic_day = match &self.picture_taking_thread {
                None => {
                    self.start_taking_pictures();
                    chrono::Local::now().day()
                }
                Some(t) => t.0.day(),
            };
            info!("Waiting pic taking to finish.");
            self.wait_taking_pictures();
            info!("Pic taking done!");
            info!("Switching pic taking folder!");
            self.curr_tmp_pic_recording_folder.switch_folders();
            info!(
                "New pic taking folder: {}",
                self.curr_tmp_pic_recording_folder.path()
            );
            info!("Starting new pic taking thread!");
            self.start_taking_pictures();
            info!("Encoding last hour!");
            self.encode_last_hour_and_move_to_today_folder_clean_pic_folder();
            let curr_pic_taking_day = self
                .picture_taking_thread
                .as_ref()
                .expect("No Pic taking thread active after starting it!")
                .0
                .day();

            if start_pic_day != curr_pic_taking_day {
                info!("New day is {}, stitching last day", start_pic_day);
                // new day, stitch last day and move it to own folder
                self.stitch();
            }
        }
    }

    fn stitch(&mut self) {
        info!("Started stitching!");
        let mut files_string = String::new();
        let structure = Self::get_dir_structure();
        if let Some(folder) = structure.today_folder {
            for movie in folder.today_movies.iter().rev() {
                files_string.push_str(&format!("file \'{}\'\n", movie.path));
            }

            let mut file = fs::File::create("files.txt").unwrap();
            file.write_all(files_string.as_bytes()).unwrap();
            // ffmpeg -f concat -safe 0 -i files.txt -c copy some.mp4
            let out_path = format!("{}/{}.mp4", ENCODING_FOLDER, folder.timestamp);
            info!("Outputting stitched result to {}!", out_path);
            let process = Command::new("ffmpeg")
                .stdout(Stdio::piped())
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
            let output = process
                .wait_with_output()
                .expect("Error waiting stitching process to end");
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                let out = String::from_utf8_lossy(&output.stdout);
                error!("Stitching process did not end successfully");
                error!("{}", err);
                error!("{}", out);
            }
            info!("Stitching done!");
            info!("Removing previous today folder!");
            fs::remove_dir_all(&folder.path).expect("Error removing today folder");
            let dest = format!("{}.mp4", &folder.path);
            info!("Moving result from {} to {}.", out_path, dest);
            fs::rename(&out_path, dest)
                .expect(&format!("Error moving {} to {}", out_path, folder.path));
        }
    }

    fn start_take_pictures_till_hour_end_thread(&mut self) -> JoinHandle<()> {
        let camera_process = self.camera.clone();
        let recording_folder = self.curr_tmp_pic_recording_folder.clone();
        let (sender, receiver) = crossbeam_channel::bounded::<PicTakingMessage>(2);
        info!("Starting new pic taking!");
        if self.picture_taking_thread.is_some() {
            panic!("Tried to start a new picture_taking_thread with one already existing!");
        }
        self.picture_taking_thread = Some((chrono::Local::now(), receiver));
        std::thread::spawn(move || {
            let local: DateTime<Local> = Local::now();
            let initial_hour = local.hour();
            info!(
                "Pic taking thread started, taking pics for hour: {}",
                initial_hour
            );
            let mut i = 0;
            // take pictures until the current hour expires or at least 5 pictures
            while (Local::now().hour() == initial_hour) || i < 5 {
                let path = format!(
                    "{}/{}/{:05}.jpg",
                    PICS_FOLDER_ROOT,
                    recording_folder.to_string(),
                    i
                );
                camera_process.take_new_pic_save_at(&path);
                i += 1;
            }
            info!("Pic taking thread done!");
            sender.send(PicTakingMessage::Done).unwrap();
        })
    }
}
