use std::fs::File;
use std::*;
use std::path::Path;
//use gif;
use rgb::RGB8;
use ansi_rgb::{Background, WithForeground};
use image::codecs::gif::GifDecoder;
use image;
use image::codecs::gif;
use image::{AnimationDecoder, GenericImageView};

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
    let file = File::open(Path::new(PATH));

    //decode to frames
    /*let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = decoder.read_info(file.unwrap()).unwrap();*/
    let mut frames = GifDecoder::new(file.unwrap()).unwrap().into_frames();
    //let frames = decoder.into_frames().expect("error decoding gif");

    //iterate frames
    for frame in frames {
        let frame = frame.unwrap();
        for line in frame.buffer().chunks(frame.buffer().width() as usize * 4) {
            //lines
            for pixel in line.chunks(4) {
                print!("{}", ".".bg(RGB8::new(pixel[0], pixel[1], pixel[2])));
            }
            print!("\n");
        }
        println!();
        thread::sleep(time::Duration::from_millis(40));
        //let image = frame.buffer.to_vec();

        /*let mut iterations: u64 = 0;
        let mut prev_pix: [u8; 4] = [1, 1, 1, 255];
        for pix in frame.buffer.to_vec().chunks(4) {
            if iterations % (64 * 64) == 0 {
                print!("{}[2J", 27 as char);
                thread::sleep(time::Duration::from_millis(200))
            }
            if iterations % 64 == 0 { println!() }
            iterations += 1;
            if pix == [0, 0, 0, 0] {
                print!("{}", ".".bg(RGB8::new(prev_pix[0], prev_pix[1], prev_pix[2])));
                continue;
            }
            print!("{}", ".".bg(RGB8::new(pix[0], pix[1], pix[2])));
            //println!("{}x{:?}", iterations, pix);
            prev_pix = <[u8; 4]>::try_from(pix).unwrap();*/


        //let mut output: WithForeground<&str> = "".fg();
        /*for pixel in frame.buffer.chunks(4) {
            iterations += 1;
            if iterations % 64 == 0 {
                println!()
            }
            print!("{}", ".".bg(RGB8::new(pixel[0], pixel[1], pixel[2])));
        }
        println!("{}x{}x{}", frame.width, frame.height, iterations / (64 * 64));

    }*/
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
