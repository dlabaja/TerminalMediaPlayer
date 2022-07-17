use std::fs::File;
use std::*;
use std::path::Path;
use image::codecs::gif::GifDecoder;
use image;
use eventual::Timer;
use std::time::Duration;
use image::{AnimationDecoder, Frame};

const PATH: &str = "/home/dlabaja/Downloads/bad_apple.gif";
const FPS :usize = 20;

fn main() {
    //get a file
    println!("Write a path");
    /*let mut file = open_file(Path::new(&get_path()));
    while file.is_err() {
        println!("{}", file.unwrap_err());
        file = open_file(Path::new(&get_path()));
    }*/
    //TODO vrátit path
    //TODO timer
    let file = File::open(Path::new(PATH));

    //decode to frames
    let frames = GifDecoder::new(file.unwrap()).unwrap().into_frames().collect_frames().unwrap().to_vec();

    //timer
    let timer = Timer::new();
    let ticks = timer.interval_ms((1000 / FPS) as u32).iter();

    //iterate frames
    let mut i = 0;
    let max_frames = frames.len();
    for _ in ticks {
        if i == max_frames - 1 { break; }
        process_frame(frames.get(i).unwrap(), i);
        i += 1;
    }
    println!("Konec");
}

fn process_frame(frame: &Frame, index: usize) {
    let mut pixels: String = "".to_string();
    for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
        for pixel in line.chunks(4) {
            pixels += &*format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]);
        }
        pixels += "\n";
    }
    //print!("{}[2J", 27 as char);
    println!("{}frame:{}/time:{:?}", pixels, index, Duration::from_secs((index / FPS) as u64));
}

fn get_path() -> String {
    let mut input = "".to_string();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn open_file(path: &Path) -> Result<File, &'static str> {
    if let Ok(i) = File::open(&path) {
        if path.is_file() && Path::new(&path).extension().unwrap_or_default() == "gif" {
            return Ok(i);
        }
    }
    Err("Invalid file or not a gif - Try again")
}
