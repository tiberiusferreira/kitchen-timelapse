#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]
#[macro_use]
extern crate rocket;
use rocket_seek_stream::SeekStream;
use std::fs;
use chrono::{DateTime, TimeZone, Timelike, Datelike, Local};
use serde::{Serialize, Deserialize};
use rocket_contrib::json::Json;
use rocket_cors::{AllowedOrigins, AllowedHeaders};


const MOVIES_FOLDER_ROOT: &str = "/mnt/skynet/movies";

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TodayMovie {
    hour: u32,
    filepath: String,
    formatted_date: String,
}

impl TodayMovie{
    pub fn new(folder_name: &str, filename: &str) -> Self {
        let date = filename_to_date(filename);
        let formatted = format!("{}h", date.hour());
        TodayMovie{
            hour: date.hour(),
            filepath: format!("{}/{}", folder_name, filename),
            formatted_date: formatted
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct PastDayMovies {
    formatted_date: String,
    timestamp: u64,
    filename: String,
}

pub fn filename_to_date(filename: &str) -> DateTime<Local>{
    assert!(
        filename.ends_with(".mp4"),
        format!("File does not end with .mp4: {}", filename)
    );
    let timestamp: u64 = filename
        .replace(".mp4", "")
        .parse::<u64>()
        .expect(&format!("File in dir was not timestamp: {}", filename));
    chrono::Local.timestamp(timestamp as i64, 0)
}
impl PastDayMovies {
    pub fn new(filename: &str) -> Self {
        let date = filename_to_date(filename);
        let formatted = format!("{}-{}-{}", date.day(), date.month(), date.year());

        Self {
            formatted_date: formatted,
            timestamp: date.timestamp() as u64,
            filename: filename.to_string(),
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct AvailableMovies {
    past_day_movies: Vec<PastDayMovies>,
    today_movies: Vec<TodayMovie>,
}

pub fn dir_curr_files() -> AvailableMovies {
    let dir = fs::read_dir(MOVIES_FOLDER_ROOT).expect("Error reading movies folder dir");
    let mut available_movies = AvailableMovies::default();
    for entry in dir {
        let entry = entry.expect("Error reading entry from movies folder dir");
        let metadata = entry.metadata().expect("No metadata for entry!");
        let file_type = metadata.file_type();
        // Files in folder it should end with .mp4
        let filename = entry.file_name().to_string_lossy().to_string();
        let is_mp4 = filename.ends_with(".mp4");
        if file_type.is_file() && is_mp4 {
            let movie = PastDayMovies::new(&filename);
            available_movies.past_day_movies.push(movie);
        }
        if file_type.is_dir(){
            let entries =
                fs::read_dir(entry.path()).expect("Error reading today_movies folder");
            for folder_entry in entries {
                let folder_entry = folder_entry.expect("Error reading entry in today folder");
                let file_type = folder_entry.file_type().expect("Error reading folder entry in today folder");
                let is_mp4 = folder_entry.file_name().to_string_lossy().ends_with(".mp4");
                let today_movie_filename = folder_entry.file_name().to_string_lossy().to_string();
                if file_type.is_file() && is_mp4 {
                    let today_movie = TodayMovie::new(&filename, &today_movie_filename);
                    available_movies.today_movies.push(today_movie);
                }
            }
        }
    }
    available_movies
}

#[get("/stream/<movie_path>")]
fn stream<'a>(movie_path: String) -> std::io::Result<SeekStream<'a>> {
    SeekStream::from_path(format!("{}/{}", MOVIES_FOLDER_ROOT, movie_path))
}

#[get("/stream/<today_folder>/<today_filename>")]
fn stream_today<'a>(today_folder: String, today_filename: String) -> std::io::Result<SeekStream<'a>> {
    SeekStream::from_path(format!("{}/{}/{}", MOVIES_FOLDER_ROOT, today_folder, today_filename))
}

#[get("/movies")]
fn movies() -> Json<AvailableMovies> {
    Json(dir_curr_files())
}

fn main() {
    let allowed_origins = AllowedOrigins::All;
    use rocket::http::Method;

    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
        .to_cors().unwrap();

    rocket::ignite().attach(cors).mount("/", routes![stream, movies, stream_today]).launch();
}
