use crate::framestream::AVFrameExt;
use color_eyre::{eyre::eyre, Result};
use framestream::FormatContextInputExt;
use image::buffer::Pixels;
use image::{ImageBuffer, Rgba};
use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::avutil::{AVFrameWithImage, AVImage};
use rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGB32;
use rsmpeg::swscale::SwsContext;
use std::env::args;
use std::ffi::CString;
use std::io::Write;

mod framestream;

fn main() -> Result<()> {
    let mut args = args();
    let bin = args.next().unwrap();
    let input = match args.next() {
        Some(arg) => arg,
        None => {
            println!("Usage {} <input video>", bin);
            return Ok(());
        }
    };
    let out_path = format!("{}.dot.txt", input);
    let path = CString::new(input)?;
    let input = AVFormatContextInput::open(&path)?;
    let frames = input.into_frames()?;

    let mut encoder = SwsContext::get_context(
        frames.info.width,
        frames.info.height,
        frames.info.format,
        frames.info.width,
        frames.info.height,
        AVPixelFormat_AV_PIX_FMT_RGB32,
        0,
    )
    .ok_or_else(|| eyre!("Failed to create encoder"))?;

    let image_buffer = AVImage::new(
        AVPixelFormat_AV_PIX_FMT_RGB32,
        frames.info.width,
        frames.info.height,
        1,
    )
    .ok_or_else(|| eyre!("Failed to allocate image buffer"))?;
    let mut target_frame = AVFrameWithImage::new(image_buffer);
    let mut last_center = None;

    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(out_path)?;

    for (i, frame) in frames.enumerate() {
        let frame = frame?;

        encoder.scale_frame(&frame, 0, frame.height, &mut target_frame)?;

        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
            frame.width as u32,
            frame.height as u32,
            target_frame.data(),
        )
        .ok_or_else(|| eyre!("Failed to get image buffer"))?;

        last_center = find_purple_dot(image.pixels(), frame.width as usize).or(last_center);
        let center = last_center.ok_or_else(|| eyre!("No purple dot found"))?;
        writeln!(&mut output, "{}, {}, {}", i, center.0, center.1)?;
        println!("{}, {}, {}", i, center.0, center.1);
    }
    Ok(())
}

fn find_purple_dot(pixel: Pixels<Rgba<u8>>, width: usize) -> Option<(usize, usize)> {
    let mut center_x = 0;
    let mut center_y = 0;
    let mut count = 0;

    for (i, pixel) in pixel.enumerate() {
        let y = i / width;
        let x = i % width;

        if pixel[0] > 215 && pixel[1] < 10 && pixel[2] > 215 {
            center_x += x;
            center_y += y;
            count += 1;
        }
    }

    if count > 0 {
        Some((center_x / count, center_y / count))
    } else {
        None
    }
}
