use std::fs::File;
use std::*;
use std::path::Path;
use rgb::RGB8;
use ansi_rgb::{Foreground, WithForeground};
use image::codecs::gif::GifDecoder;
use image;

use image::AnimationDecoder;

const PATH: &str = "/home/dlabaja/Downloads/bad_apple.gif";

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
    let frames = GifDecoder::new(file.unwrap()).unwrap().into_frames();

    //iterate frames
    let mut i = 0;
    for frame in frames {
        i += 1;
        let frame = frame.unwrap();
        let mut pixels: String = "".to_string();
        for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
            for pixel in line.chunks(4) {
                pixels += &*format!("\x1b[38;2;{};{};{}m██", pixel[0], pixel[1], pixel[2]);
            }
            pixels += "\n";
        }
        print!("{}[2J", 27 as char);
        println!("{}\n{}",pixels, i);
        thread::sleep(time::Duration::from_millis(40));
    }
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
