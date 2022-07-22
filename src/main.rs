use std::fs::{DirBuilder, File};
use std::*;
use std::fmt::format;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use eventual::Timer;
use image::{AnimationDecoder, Frame};
use std::process::Command;
use std::time::Duration;
use rodio::{Decoder, OutputStream, Source};

const FPS: usize = 20;
const VIDEO_FORMATS: [&str; 9] = ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "flw"];

fn main() {
    //TODO pauza tlačítko, max terminal

    //crossterm::terminal::SetSize();

    //check ffmpeg
    Command::new("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    //open file
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 { panic!("Expected 1 argument, got {}! Hint - add filepath as the argument", args.len() - 1) }
    let path = Path::new(args[1].trim());
    if File::open(path).is_err() || !is_video(path) { panic!("Invalid path or unsupported file!") }

    println!("Path is right - processing video (might take some time)");

    //upsert cache folder
    let cache_folder = &format!("{}{}TerminalMediaPlayer", dirs::video_dir().unwrap().display(), get_system_backslash());
    if File::open(cache_folder).is_err() {
        DirBuilder::new().create(cache_folder).expect(&*format!("Unable to create cache folder in {}", dirs::video_dir().unwrap().display()));
    }

    //convert video
    println!("Converting video");
    /*let new_path = &format!("{}{}{}.mp4", cache_folder, get_system_backslash(), path.file_stem().unwrap().to_str().unwrap());
    Command::new("ffmpeg").args(["-i", &path.display().to_string(), "-vf", "fps=20", new_path])
        .output().unwrap_or_else(|_| panic!("Ffmpeg can't convert the video"));
    let path = Path::new(new_path);*/

    let video = &format!("{}{}{}.gif", cache_folder, get_system_backslash(), path.file_stem().unwrap().to_str().unwrap());
    ffmpeg_handler(vec!["-vf", &format!("scale={}:{},fps={}", term_size::dimensions().unwrap().0.clamp(16, 192) / 2, term_size::dimensions().unwrap().1.clamp(9, 54), FPS)],
                   path.to_str().unwrap(), video);

    //convert audio
    println!("Converting audio");
    let audio = format!("{}{}{}.mp3", cache_folder, get_system_backslash(), path.file_stem().unwrap().to_str().unwrap());
    ffmpeg_handler(vec![], path.to_str().unwrap(), &audio);

    //decode to frames
    println!("Processing frames");
    let mut frames = GifDecoder::new(File::open(video).unwrap()).unwrap().into_frames().collect_frames().unwrap();

    //timer
    let timer = Timer::new();
    let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

    //iterate frames
    //let max_frames = frames.len();

    //play audio
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();


    for (i, _) in ticks.enumerate() {
        if i == 10 {
            let source = Decoder::new(File::open(&audio).unwrap()).unwrap();
            stream_handle.play_raw(source.convert_samples()).expect("TODO: panic message");
        }
        if frames.get(0).is_none() { break; }
        process_frame(frames.get(0).unwrap(), i);
        frames.remove(0);
    }
    println!("End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer");
}

fn ffmpeg_handler(ffmpeg_args: Vec<&str>, input_path: &str, output_path: &str) {
    if File::open(output_path).is_err() {
        let mut args = vec!["-i", input_path];
        for arg in ffmpeg_args {
            args.push(arg);
        }
        args.append(&mut vec![output_path, "-y"]);

        Command::new("ffmpeg").args(args)
            .output().unwrap_or_else(|_| panic!("Ffmpeg can't convert the video from {} to {}", input_path, output_path));
        return;
    }
    println!("Video found in cache ({}), aborting conversion", output_path)
}

fn is_video(path: &Path) -> bool {
    if path.is_dir() {
        return false;
    }
    if VIDEO_FORMATS.contains(&path.extension().unwrap().to_str().unwrap()) {
        return true;
    }
    false
}

fn get_system_backslash() -> &'static str {
    if cfg!(windows) {
        return "\\";
    }
    "/"
}

fn process_frame(frame: &Frame, index: usize) {
    let mut pixels: String = "".to_string();
    for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
        for pixel in line.chunks(4) {
            pixels += &*format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]);
        }
        pixels += "\n";
    }
    print!("{}[2J", 27 as char);
    println!("{}\x1b[38;2;255;255;255mFrame={}/Time={}s", pixels, index, secs_to_secs_and_mins(index / FPS));
}

fn secs_to_secs_and_mins(secs: usize) -> String {
    let mins = ((secs / 60) as f32).floor();
    let seconds = secs - (mins as i32 * 60) as usize;
    if seconds < 10 {
        return format!("{}:0{}", mins, seconds);
    }
    format!("{}:{}", mins, seconds)
}
