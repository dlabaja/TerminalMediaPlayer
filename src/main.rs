use std::fs::File;
use std::*;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use eventual::Timer;
use image::{AnimationDecoder, Frame};
use std::process::Command;
use rodio::{Decoder, OutputStream, Source};

const FPS: usize = 20;
const VIDEO_FORMATS: [&str; 9] = ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "flw"];

fn main() {
    //check ffmpeg
    Command::new("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    //open file
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {panic!("Expected 1 argument, got {}! Hint - add filepath as the argument", args.len() - 1)}
    let path = Path::new(args[1].trim());
    if File::open(path).is_err() || !is_video(path) {panic!("Invalid path or unsupported file!")}

    println!("Path is right - processing video (might take some time)");

    //convert video
    println!("Converting video");
    let video = &format!("{}{}output.gif", dirs::data_local_dir().unwrap().display(), get_system_backslash());
    Command::new("ffmpeg").args(["-i", &path.display().to_string(), "-vf", &format!("scale={}:{},fps=20", term_size::dimensions().unwrap().1.clamp(16, 96), term_size::dimensions().unwrap().0.clamp(9, 54)), &format!("{}", Path::new(video).display()), "-y"]).output().expect("Unable to convert to gif");

    //convert audio
    println!("Converting audio");
    let audio = &format!("{}{}output.mp3", dirs::data_local_dir().unwrap().display(), get_system_backslash());
    Command::new("ffmpeg").args(["-i", &path.display().to_string(), &format!("{}", Path::new(audio).display()), "-y"]).output().expect("Unable to convert audio");

    //decode to frames
    println!("Processing frames");
    let frames = GifDecoder::new(File::open(video).unwrap()).unwrap().into_frames().collect_frames().unwrap();

    //timer
    let timer = Timer::new();
    let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

    //iterate frames
    let max_frames = frames.len();

    //play audio
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let source = Decoder::new(File::open(audio).unwrap()).unwrap();
    stream_handle.play_raw(source.convert_samples()).expect("TODO: panic message");

    for (i, _) in ticks.enumerate() {
        if i == max_frames - 1 { break; }
        process_frame(frames.get(i).unwrap(), i);
    }
    println!("End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer");
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
    println!("{}\x1b[38;2;255;255;255mframe={}/time={}s", pixels, index, secs_to_secs_and_mins(index / FPS));
}

fn get_path() -> String {
    let mut input = "".to_string();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn secs_to_secs_and_mins(secs: usize) -> String {
    let mins = ((secs / 60) as f32).floor();
    let seconds = secs - (mins as i32 * 60) as usize;
    if seconds < 10 {
        return format!("{}:0{}", mins, seconds);
    }
    format!("{}:{}", mins, seconds)
}
