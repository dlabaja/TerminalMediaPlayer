use std::fs::File;
use std::*;
use std::path::Path;
use rgb::RGB8;
use ansi_rgb::{Foreground, WithBackground, WithForeground};
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
    let mut frames = GifDecoder::new(file.unwrap()).unwrap().into_frames();

    //iterate frames
    for frame in frames {
        let frame = frame.unwrap();
        let mut pixels: Vec<WithForeground<&str>> = Vec::new();

        for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
            for pixel in line.chunks(4) {
                pixels.push("██".fg(RGB8::new(pixel[0], pixel[1], pixel[2])));
            }
            pixels.push("\n".fg(RGB8::new(0, 0, 0)));
        }

        for i in pixels {
            print!("{}", i);
        }
        println!();
        thread::sleep(time::Duration::from_millis(40));
    }
}

fn get_path() -> String {
    let mut input = "".to_string();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn open_file(path: &Path) -> std::result::Result<File, &'static str> {
    if let Ok(i) = File::open(&path) {
        if path.is_file() && Path::new(&path).extension().unwrap_or_default() == "gif" {
            return Ok(i);
        }
    }
    Err("Invalid file or not a gif - Try again")
}
