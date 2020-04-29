//raspistill -st -q 7 -w 1640 -h 1232 -t 300000 -tl 700 -n -ex sports -o image7_%04d.jpg
//ffmpeg -framerate 10 -i image7_%04d.jpg -video_size 1640:1232 -c:v h264_omx -bufsize 64k -b:v 1.2M -vf fps=10 out.mp4
use flexi_logger::{Cleanup, Criterion, Naming};
mod camera_api;

mod timelapse;

fn main() {
    use flexi_logger::colored_opt_format;
    use log::info;
    flexi_logger::Logger::with_str("info")
        .format(colored_opt_format)
        .log_to_file()
        .directory("./logs")
        .rotate(
            Criterion::Size(500_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(2),
        )
        .start()
        .unwrap();
    info!("Starting up...");
    let mut timelapse_manufacturer = timelapse::TimeLapseManufacturer::new();
    timelapse_manufacturer.run();
}
