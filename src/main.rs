use std::fs::{DirBuilder, File};
use std::*;
use std::io::stdout;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use eventual::{Async, Timer};
use image::{AnimationDecoder, Frame};
use std::process::Command;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, read};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use humthreads::{Builder, Thread};
use rodio::{Decoder, OutputStream, Source};
use lazy_static::lazy_static;

const FPS: usize = 20;
const VIDEO_FORMATS: [&str; 9] = ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "flw"];

lazy_static!(
    static ref CACHE: Mutex<bool> = Mutex::new(true);
    static ref THREAD: Mutex<Thread<()>> = spawn_input_thread();
);

fn main() {
    //panic setup
    panic::set_hook(Box::new(|info| {
        let info = info.to_string();
        println!("There was an error - {}", info.split("'").collect::<Vec<&str>>()[1]);
        end_program();
    }));

    //check ffmpeg
    Command::new("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    //open file
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
    ffmpeg_handler(vec!["-vf", &format!("scale={}:{},fps={}", term_size::dimensions().unwrap().0.clamp(32, 196) / 2, term_size::dimensions().unwrap().1.clamp(9, 54), FPS)],
                   &path, video);

    //convert audio
    println!("Converting audio");
    let audio = format!("{}{}{}.mp3", cache_folder, get_system_backslash(), Path::new(&path).file_stem().unwrap().to_str().unwrap());
    ffmpeg_handler(vec![], &path, &audio);

    //decode to frames
    println!("Processing frames");
    let mut frames = GifDecoder::new(File::open(video).unwrap()).unwrap().into_frames().collect_frames().unwrap();

    //timer
    let timer = Timer::new();
    let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

    //iterate frames
    for (i, _) in ticks.enumerate() {
        if i == 10 {
            play_audio(Decoder::new(File::open(&audio).unwrap()).unwrap());
        }
        if frames.get(0).is_none() { break; }
        process_frame(frames.get(0).unwrap(), i);
        frames.remove(0);
    }
    println!("End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer");
    end_program();
}

fn play_audio(source: Decoder<File>) {
    thread::spawn(|| {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        stream_handle.play_raw(source.convert_samples()).expect("TODO: panic message");
        thread::sleep(Duration::from_secs(u64::MAX));
    });
}

fn spawn_input_thread() -> Mutex<Thread<()>> {
    let thread = Builder::new("input".to_string()).spawn(|thread| {
        loop {
            while !thread.should_shutdown() {
                enable_raw_mode().expect("\n");
                let input = read().unwrap();
                match input {
                    Event::Key(KeyEvent {
                                   code: KeyCode::Char('p'),
                                   modifiers: KeyModifiers::NONE,
                               }) => on_pause(),
                    Event::Key(KeyEvent {
                                   code: KeyCode::Char('c'),
                                   modifiers: KeyModifiers::CONTROL,
                               }) => end_program(),
                    _ => (),
                }
                disable_raw_mode().unwrap();
            }
        }
    }).expect("Cannot create new thread");
    Mutex::new(thread)
}

fn on_pause() {
    Print("pressed p");
}

fn get_file_path() -> String {
    let args: Vec<String> = env::args().collect();

    let path = &args.get(1).expect(&format!("Expected 1 argument, got {}! Hint - add filepath as the argument", args.len() - 1)).trim();
    if args.contains(&"--ignore-cache".to_string()) {
        *CACHE.lock().unwrap() = false;
    }

    if File::open(Path::new(path)).is_err() || !is_video(Path::new(path)) { panic!("Invalid path or unsupported file!") }
    return path.to_string();
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

fn end_program() {
    THREAD.lock().unwrap().request_shutdown();
    disable_raw_mode().unwrap();
    process::exit(0);
}
