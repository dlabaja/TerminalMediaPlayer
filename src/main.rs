use std::fs::{DirBuilder, File};
use std::*;
use std::io::BufReader;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use image;
use eventual::Timer;
use image::{AnimationDecoder, Frame};
use std::process::Command;
use dirs;
use rodio::{Decoder, OutputStream, Source};

const PATH: &str = "/home/dlabaja/Downloads/kscm.webm";
const FPS: usize = 20;

fn main() {
    let shell_alias = if cfg!(windows) { "cmd" } else { "sh" };

    //check ffmpeg
    Command::new("sh").arg("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    //get a file
    println!("Write a path");
    let mut file = open_file(Path::new(&get_path()));
    while file.is_err() {
        println!("{}", file.unwrap_err());
        file = open_file(Path::new(&get_path()));
    }

    println!("Path is right - processing video (might take some time)");

    //convert video
    println!("Converting video");
    let video = &format!("{}{}output.gif", dirs::data_local_dir().unwrap().display(), get_system_backslash());
    Command::new("ffmpeg").args(["-i", PATH, "-vf", "scale=96:54,fps=20", &format!("{}", Path::new(video).display()), "-y"]).output().expect("Unable to convert to gif");

    //convert audio
    println!("Converting audio");
    let audio = &format!("{}{}output.mp3", dirs::data_local_dir().unwrap().display(), get_system_backslash());
    Command::new("ffmpeg").args(["-i", PATH, &format!("{}", Path::new(audio).display()), "-y"]).output().expect("Unable to convert audio");

    //decode to frames
    println!("Processing frames");
    let frames = GifDecoder::new(File::open(video).unwrap()).unwrap().into_frames().collect_frames().unwrap();

    //timer
    let timer = Timer::new();
    let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

    //iterate frames
    let mut i = 0;
    let max_frames = frames.len();

    //play audio
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let source = Decoder::new(File::open(audio).unwrap()).unwrap();
    stream_handle.play_raw(source.convert_samples()).expect("TODO: panic message");

    for _ in ticks {
        if i == max_frames - 1 { break; }
        process_frame(frames.get(i).unwrap(), i);
        i += 1;
    }
    println!("End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer");
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

fn open_file(path: &Path) -> Result<File, &'static str> {
    if let Ok(i) = File::open(&path) {
        if path.is_file() && Path::new(&path).extension().unwrap_or_default() == "gif" {
            return Ok(i);
        }
    }
    Err("Invalid file or not a gif - Try again")
}
