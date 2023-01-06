use std::fs::{DirBuilder, File, read_dir};
use std::*;
use eventual::Timer;
use std::error::Error;
use std::io::{BufReader, Cursor, stdout, Stdout, Write};
use std::path::Path;
use image::{DynamicImage, RgbImage};
use std::process::{Command, Output};
use std::sync::Mutex;
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read};
use crossterm::{cursor, event, ExecutableCommand, execute, queue, QueueableCommand, style, terminal};
use crossterm::style::Colored::ForegroundColor;
use crossterm::style::{Color, Print, SetForegroundColor};
use crossterm::terminal::{ClearType, disable_raw_mode, enable_raw_mode};
use rodio::{Decoder, OutputStream, Sink};
use lazy_static::lazy_static;
use image::io::Reader as ImageReader;

const FPS: usize = 20;
const VIDEO_FORMATS: [&str; 9] = ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "flw"];

lazy_static!(
    static ref IS_PLAYING: Mutex<bool> = Mutex::new(true);
    static ref FRAMES: Mutex<Vec<DynamicImage>> = Mutex::new(vec!());
    static ref VOLUME: Mutex<f32> = Mutex::new(1f32);
);

fn main() {
    //TODO fixnout ignore-cache, --no-ribbon, moc velký aspect ratio

    //panic setup
    panic::set_hook(Box::new(|info| {
        end_process(info.to_string());
        //end_process(info.to_string().split('\'').collect::<Vec<&str>>()[1].to_string());
    }));

    //parse args
    let args: Vec<String> = env::args().collect();

    //get valid path
    let path = Path::new(args.get(1).unwrap_or_else(|| panic!("Expected 1 argument, got {}! Hint - add filepath as the argument", args.len() - 1)).trim());
    if File::open(Path::new(path)).is_err() || !is_video(Path::new(path)) { panic!("Invalid path or unsupported file!") }

    //get max_frames
    let max_frames = get_max_frames(Path::new(&path));

    //check ffmpeg
    Command::new("ffmpeg").output().expect("FFMPEG NOT FOUND! Please install one at https://ffmpeg.org/");

    println!("Processing video (this might take some time)\n------------------");

    //upsert TMP & CACHE folder
    let tmp_folder = format!("{}{}TerminalMediaPlayer", dirs::data_dir().unwrap().display(), get_system_backslash());
    if !Path::new(&tmp_folder).exists() {
        println!("{}", File::open(&tmp_folder).is_err());
        DirBuilder::new().create(&tmp_folder).expect(&*format!("Unable to create {} folder", tmp_folder));
    }
    let cache_folder = format!("{}{}{}", tmp_folder, get_system_backslash(), Path::new(&path).file_stem().unwrap().to_str().unwrap());
    let ignore_cache = if args.contains(&"--ignore-cache".to_string()) || !Path::new(&cache_folder).exists() { true } else { false };
    if !Path::new(&cache_folder).exists() {
        DirBuilder::new().create(&cache_folder).expect(&*format!("Unable to create {} folder", cache_folder));
    }

    let audio = format!("{}{}{}.mp3", &cache_folder, get_system_backslash(), Path::new(&path).file_stem().unwrap().to_str().unwrap());
    if ignore_cache {
        //convert video
        println!("Converting video");
        fs::remove_dir_all(&cache_folder).unwrap();
        fs::create_dir(&cache_folder).unwrap();
        convert_video(cache_folder.clone(), path.to_str().unwrap().to_string());
        thread::sleep(Duration::from_millis(3000));

        //convert audio
        println!("Converting audio");
        ffmpeg_handler(vec![&audio], path.to_str().unwrap());
    }

    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    queue!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::DisableBlinking,
            cursor::Hide,
            ).unwrap();
    stdout.flush().expect("TODO: panic message");

    //input thread
    thread::spawn(|| {
        loop {
            if poll(Duration::from_secs(0)).unwrap() {
                match read().unwrap() {
                    Event::Key(KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::NONE, .. }) =>
                        on_pause(),
                    Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, .. }) =>
                        on_volume_up(),
                    Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, .. }) =>
                        on_volume_down(),
                    Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) =>
                        end_process("Process exited successfully".to_string()),
                    _ => ()
                }
            }
        }
    });

    //push to buffer
    thread::spawn(move || {
        let mut iter = 1;
        loop {
            for _ in 0..10 {
                let cur_file = format!("{}{}{}.png", cache_folder, get_system_backslash(), iter.to_string().trim());
                if ImageReader::open(&cur_file).is_err() { return; }
                let img = ImageReader::open(&cur_file).unwrap().decode().unwrap();
                FRAMES.lock().unwrap().push(img);
                iter += 1;
            }
            thread::sleep(Duration::from_millis(300));
        }
    });

    //iterate frames
    let mut current_frame = 0;
    loop {
        while !*IS_PLAYING.lock().unwrap() {}; //wait on unpause

        let timer = Timer::new();
        let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

        for _ in ticks.enumerate() {
            if !*IS_PLAYING.lock().unwrap() { break; }

            if current_frame == 10 {
                play_audio(File::open(&audio).unwrap());
            }

            generate_frame(FRAMES.lock().expect("amog").get(0)
                .expect("End of playback\nhttps://github.com/dlabaja/TerminalMediaPlayer").to_rgb8());
            generate_ribbon(current_frame + 1, max_frames);
            FRAMES.lock().expect("amog").remove(0);
            current_frame += 1;
        }
    }
}

fn end_process(msg: String) {
    disable_raw_mode().expect("TODO: panic message");
    let mut stdout = stdout();
    queue!(
            stdout,
            terminal::LeaveAlternateScreen,
            cursor::EnableBlinking,
            cursor::Show,
            style::SetForegroundColor(Color::White)
        ).unwrap();
    stdout.flush().unwrap();
    println!("{}", msg);
    process::exit(0);
}

fn convert_video(cache_folder: String, path: String) {
    //get video size
    let aspect_ratio = String::from_utf8(Command::new("ffprobe").args([&path, "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=display_aspect_ratio", "-of", "csv=s=x:p=0"]).
        output().unwrap().stdout).unwrap();
    let aspect_ratio: Vec<&str> = aspect_ratio.trim().split(':').into_iter().collect();
    let (width, height) = get_ideal_resolution(aspect_ratio[0].parse::<usize>().unwrap() as f32, aspect_ratio[1].parse::<usize>().unwrap() as f32);

    thread::spawn(move || {
        ffmpeg_handler(vec!["-vf", &format!("scale={}:{},fps={}", &width, &height, FPS), &format!("{}{}%0d.png", &cache_folder, get_system_backslash())], &path);
    });
}

fn generate_frame(frame: RgbImage) {
    let mut stdout = stdout();
    let mut y = 0;

    for line in frame.chunks(frame.width() as usize * 3) {
        let mut pixels = "".to_string();
        for pixel in line.chunks(3) {
            pixels += &*format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]);
        }
        stdout.queue(cursor::MoveTo(0, y)).unwrap().queue(Print(&pixels)).unwrap();
        y += 1;
    }
    stdout.flush().unwrap();
}

fn get_max_frames(path: &Path) -> usize {
    let cur_max_frames = String::from_utf8(Command::new("ffprobe").args(format!("-v error -select_streams v:0 -count_packets -show_entries stream=nb_read_packets -of csv=p=0 {}", path.to_str().unwrap()).split(' ')).
        output().unwrap().stdout).unwrap().trim().parse::<usize>().unwrap();
    let cur_fps = String::from_utf8(Command::new("ffprobe").args(format!("-v error -select_streams v -of default=noprint_wrappers=1:nokey=1 -show_entries stream=r_frame_rate {}", path.to_str().unwrap()).split(' ')).
        output().unwrap().stdout).unwrap();
    let cur_fps = cur_fps.trim().split('/').collect::<Vec<&str>>();
    let fps_ratio = cur_fps[0].parse::<usize>().unwrap() / cur_fps[1].parse::<usize>().unwrap();

    (cur_max_frames as f32 / (fps_ratio as f32 / FPS as f32)) as usize
}

fn get_ideal_resolution(width: f32, height: f32) -> (usize, usize) {
    let mut amplifier: f32 = 0.0;
    let term_width = terminal::size().unwrap().0;
    let term_height = terminal::size().unwrap().1 - 5;
    for i in 0..1000
    {
        let i = (i as f32) / 10.0;
        if width * i * 2.0 > term_width as f32 || height * i > term_height as f32 {
            break;
        }
        amplifier = i;
    }
    ((width * amplifier).round() as usize, (height * amplifier).round() as usize)
}

fn ffmpeg_handler(ffmpeg_args: Vec<&str>, input_path: &str) -> Output {
    let mut args = vec!["-i", input_path];
    for arg in ffmpeg_args {
        args.push(arg);
    }
    args.push("-y");

    Command::new("ffmpeg").args(args)
        .output().unwrap_or_else(|_| panic!("FFMPEG failed, aborting"))
}

fn on_volume_up() {
    let volume = *VOLUME.lock().unwrap();
    if (volume * 10.0).round() / 10.0 < 6.0 {
        *VOLUME.lock().unwrap() += 0.1;
        *VOLUME.lock().unwrap() = ((volume + 0.1) * 10.0).round() / 10.0;
    }
}

fn on_volume_down() {
    let volume = *VOLUME.lock().unwrap();
    if (volume * 10.0).round() / 10.0 > 0.0 {
        *VOLUME.lock().unwrap() -= 0.1;
        *VOLUME.lock().unwrap() = ((volume - 0.1) * 10.0).round() / 10.0;
    }
}

fn generate_ribbon(index: usize, max_frames: usize) {
    let mut stdout = stdout();
    queue!(stdout,
        cursor::MoveTo(0, cursor::position().unwrap().1 + 1),
        Print(format!("\x1b[38;2;255;255;255m{}s <{}> {}s\r\nFrame={}/{}  Volume={:.0}% \r\nPress 'P' to pause/play  Press 'ArrowUp/Down' to change volume\r\n\
            If the screen freezes, restart the program using CTRL+C",
            secs_to_secs_and_mins(index / FPS), generate_timeline(index, max_frames), secs_to_secs_and_mins(max_frames / FPS), index, max_frames, *VOLUME.lock().unwrap() * 100.0))).unwrap();
}

fn generate_timeline(index: usize, max_frames: usize) -> String {
    let part_count = 20;
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
            sink.set_volume(*VOLUME.lock().unwrap())
        }
    });
}

fn on_pause() {
    let is_playing = *IS_PLAYING.lock().unwrap();
    if !is_playing {
        *IS_PLAYING.lock().unwrap() = true;
        return;
    }
    *IS_PLAYING.lock().unwrap() = false;
}

fn is_video(path: &Path) -> bool {
    if path.is_dir() {
        return false;
    }
    if VIDEO_FORMATS.contains(&path.extension().expect("sus").to_str().expect("amogus")) {
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

fn secs_to_secs_and_mins(secs: usize) -> String {
    let mins = ((secs / 60) as f32).floor();
    let seconds = secs - (mins as i32 * 60) as usize;
    if seconds < 10 {
        return format!("{}:0{}", mins, seconds);
    }
    format!("{}:{}", mins, seconds)
}