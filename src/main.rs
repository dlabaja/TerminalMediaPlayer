use std::fs::{DirBuilder, File};
use std::*;
use std::io::BufReader;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use eventual::Timer;
use image::{AnimationDecoder, Frame};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use rodio::{Decoder, OutputStream, Sink};
use lazy_static::lazy_static;

const FPS: usize = 20;
const VIDEO_FORMATS: [&str; 9] = ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "flw"];

lazy_static!(
    static ref CACHE: Mutex<bool> = Mutex::new(true);
    static ref IS_PLAYING: Mutex<bool> = Mutex::new(true);
);

fn main() {
    //panic setup
    panic::set_hook(Box::new(|info| {
        println!("{}", info.to_string().split('\'').collect::<Vec<&str>>()[1]);
    }));

    //check ffmpeg
    Command::new("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    //get valid path
    let path = get_file_path();

    println!("Processing video (this might take some time)");

    //upsert CACHE folder
    let cache_folder = &format!("{}{}TerminalMediaPlayer", dirs::video_dir().unwrap().display(), get_system_backslash());
    if File::open(cache_folder).is_err() {
        DirBuilder::new().create(cache_folder).expect(&*format!("Unable to create CACHE folder in {}", dirs::video_dir().unwrap().display()));
    }

    //convert video
    println!("Converting video");
    let video = &format!("{}{}{}.gif", cache_folder, get_system_backslash(), Path::new(&path).file_stem().unwrap().to_str().unwrap());
    ffmpeg_handler(vec!["-vf", &format!("scale={}:{},fps={}", crossterm::terminal::size().unwrap().0.clamp(32, 196) / 2, crossterm::terminal::size().unwrap().1.clamp(9, 54), FPS)],
                   &path, video);

    //convert audio
    println!("Converting audio");
    let audio = format!("{}{}{}.mp3", cache_folder, get_system_backslash(), Path::new(&path).file_stem().unwrap().to_str().unwrap());
    ffmpeg_handler(vec![], &path, &audio);

    //decode to frames
    println!("Processing frames");
    let mut frames = GifDecoder::new(File::open(video).unwrap()).unwrap().into_frames().collect_frames().unwrap();

    //input thread
    thread::spawn(|| {
        let timer = Timer::new();
        let ticks = timer.interval_ms((1000 / FPS) as u32).iter();
        for _ in ticks.enumerate() {
            enable_raw_mode().unwrap();
            if poll(Duration::from_secs(0)).unwrap() {
                if let Event::Key(KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::NONE, }) = read().unwrap() {
                    on_pause()
                }
            }
            disable_raw_mode().unwrap();
        }
    });

    //iterate frames
    let mut current_frame = 0;
    let max_frames = frames.len();
    loop {
        while !*IS_PLAYING.lock().unwrap() {}; //wait on unpause

        let timer = Timer::new();
        let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

        for _ in ticks.enumerate() {
            if !*IS_PLAYING.lock().unwrap() { break; }

            if current_frame == 10 {
                play_audio(File::open(&audio).unwrap());
            }

            println!("{}[2J{}", 27 as char, generate_frame(frames.get(0).expect(
                "End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer")));
            println!("{}", generate_ribbon(current_frame, max_frames));
            frames.remove(0);
            current_frame += 1;
        }
    }
}

fn generate_ribbon(index: usize, max_frames: usize) -> String {
    format!("\x1b[38;2;255;255;255m{}s <{}> {}s\nFrame={}/{}    Press 'P' to pause/play", secs_to_secs_and_mins(index / FPS), generate_timeline(index, max_frames), secs_to_secs_and_mins(max_frames / FPS), index, max_frames)
}

fn generate_timeline(index: usize, max_frames: usize) -> String {
    let part_count = 15;
    let mut timeline = "".to_string();
    for _ in 0..f64::floor((index as f64 / f64::floor((max_frames / part_count) as f64)) as f64) as i32 {
        timeline += "=";
    }
    timeline += "|";
    while timeline.chars().count() < part_count + 1 {
        timeline += "-";
    }
    timeline
}

fn play_audio(file: File) {
    thread::spawn(|| {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let source = Decoder::new(BufReader::new(file)).unwrap();
        sink.append(source);
        loop {
            while !*IS_PLAYING.lock().unwrap() {
                sink.pause();
            }
            sink.play();
        }
    });
}

fn on_pause() {
    disable_raw_mode().unwrap();
    let is_playing = *IS_PLAYING.lock().unwrap();
    if !is_playing {
        *IS_PLAYING.lock().unwrap() = true;
        return;
    }
    *IS_PLAYING.lock().unwrap() = false;
}

fn get_file_path() -> String {
    let args: Vec<String> = env::args().collect();

    let path = &args.get(1).unwrap_or_else(|| panic!("Expected 1 argument, got {}! Hint - add filepath as the argument", args.len() - 1)).trim();
    if args.contains(&"--ignore-cache".to_string()) {
        *CACHE.lock().unwrap() = false;
    }

    if File::open(Path::new(path)).is_err() || !is_video(Path::new(path)) { panic!("Invalid path or unsupported file!") }
    path.to_string()
}

fn ffmpeg_handler(ffmpeg_args: Vec<&str>, input_path: &str, output_path: &str) {
    if File::open(output_path).is_err() || !*CACHE.lock().unwrap() {
        let mut args = vec!["-i", input_path];
        for arg in ffmpeg_args {
            args.push(arg);
        }
        args.append(&mut vec![output_path, "-y"]);

        Command::new("ffmpeg").args(args)
            .output().unwrap_or_else(|_| panic!("Ffmpeg can't convert the video from {} to {}", input_path, output_path));
        return;
    }
    println!("Video found in CACHE ({}), aborting conversion. If you want to convert anyways, use \"--ignore-cache\" flag", output_path)
}

fn is_video(path: &Path) -> bool {
    if path.is_dir() {
        return false;
    }
    println!("{:?}", path);
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

fn generate_frame(frame: &Frame) -> String {
    let mut pixels = "".to_string();
    for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
        for pixel in line.chunks(4) {
            pixels += &*format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]);
        }
        pixels += "\n";
    }
    pixels
}

fn secs_to_secs_and_mins(secs: usize) -> String {
    let mins = ((secs / 60) as f32).floor();
    let seconds = secs - (mins as i32 * 60) as usize;
    if seconds < 10 {
        return format!("{}:0{}", mins, seconds);
    }
    format!("{}:{}", mins, seconds)
}
