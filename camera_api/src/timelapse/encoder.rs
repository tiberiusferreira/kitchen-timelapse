use std::process::{Child, Command};
use crate::timelapse::{EncodingMessage, TimeLapseManufacturer, EncodingOutput};


impl TimeLapseManufacturer {
    // fn state(&self) -> EncoderState {
    //     self.state.clone()
    // }

    pub fn start_encoding_thread(&mut self, img_dir: String, output_path_with_filename: String, filename: String) {
        let (sender, receiver) = crossbeam_channel::bounded::<EncodingMessage>(2);
        self.encoding_thread = Some(receiver);
        std::thread::spawn(move ||{
            let mut process = Command::new("ffmpeg")
                //-i image%04d.jpg -video_size 1640:1232 -c:v h264_omx -b:v 1.2M -vf fps=10 out.mp4
                .arg("-framerate")
                .arg("10")
                .arg("-i")
                .arg(format!("{}/%05d.jpg", img_dir))
                .arg("-video_size")
                .arg("1640:1232")
                .arg("-vf")
                .arg("fps=10")
                .arg(output_path_with_filename.clone())
                .spawn()
                .expect("command failed to start");
            process.wait().unwrap();
            let encoding_output = EncodingOutput{
                output_path: output_path_with_filename,
                filename
            };
            sender.send(EncodingMessage::Done(encoding_output))
        });

    }
}