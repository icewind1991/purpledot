//! Port from Original code: https://github.com/leandromoreira/ffmpeg-libav-tutorial/blob/master/0_hello_world.c

use rusty_ffmpeg::ffi;
use rusty_ffmpeg::ffi::{
    AVCodec, AVCodecContext, AVFormatContext, AVFrame, AVPacket, AVPixelFormat_AV_PIX_FMT_RGB24,
    AVPixelFormat_AV_PIX_FMT_YUV420P,
};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use std::{
    ffi::{CStr, CString},
    fs::File,
    io::Write,
    ptr, slice,
};

pub struct RawFrameIter {
    frame_context: OwnedAvFormatContext,
    packet: OwnedAvPacket,
    frame: OwnedAvFrame,
    codec_context: OwnedAvCodecContext,
}

impl RawFrameIter {
    pub fn new(filepath: CString) -> Self {
        println!("initializing all the containers, codecs and protocols.");

        let mut format_context = OwnedAvFormatContext::new();

        println!(
            "opening the input file ({}) and loading format (container) header",
            filepath.to_str().unwrap()
        );

        if unsafe {
            ffi::avformat_open_input(
                &mut (format_context.deref_mut() as *mut _) as *mut *mut _,
                filepath.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } != 0
        {
            panic!("ERROR could not open the file");
        }

        let format_name = unsafe { CStr::from_ptr((*format_context.iformat).name) }
            .to_str()
            .unwrap();

        println!(
            "format {}, duration {} us, bit_rate {}",
            format_name, format_context.duration, format_context.bit_rate
        );

        println!("finding stream info from format");

        if unsafe { ffi::avformat_find_stream_info(format_context.deref_mut(), ptr::null_mut()) }
            < 0
        {
            panic!("ERROR could not get the stream info");
        }

        let mut codec_ptr: *const ffi::AVCodec = ptr::null_mut();
        let mut codec_parameters_ptr: *const ffi::AVCodecParameters = ptr::null_mut();
        let mut video_stream_index = None;
        let mut resolution = None;

        let streams = unsafe {
            slice::from_raw_parts(format_context.streams, format_context.nb_streams as usize)
        };

        for (i, stream) in streams
            .iter()
            .map(|stream| unsafe { stream.as_ref() }.unwrap())
            .enumerate()
        {
            println!(
                "AVStream->time_base before open coded {}/{}",
                stream.time_base.num, stream.time_base.den
            );
            println!(
                "AVStream->r_frame_rate before open coded {}/{}",
                stream.r_frame_rate.num, stream.r_frame_rate.den
            );
            println!("AVStream->start_time {}", stream.start_time);
            println!("AVStream->duration {}", stream.duration);
            println!("finding the proper decoder (CODEC)");

            let local_codec_params = unsafe { stream.codecpar.as_ref() }.unwrap();
            let local_codec =
                unsafe { ffi::avcodec_find_decoder(local_codec_params.codec_id).as_ref() }
                    .expect("ERROR unsupported codec!");

            match local_codec_params.codec_type {
                ffi::AVMediaType_AVMEDIA_TYPE_VIDEO => {
                    if video_stream_index.is_none() {
                        video_stream_index = Some(i);
                        codec_ptr = local_codec;
                        codec_parameters_ptr = local_codec_params;
                    }

                    println!(
                        "Video Codec: resolution {} x {}",
                        local_codec_params.width, local_codec_params.height
                    );
                    resolution = Some((local_codec_params.width, local_codec_params.height));
                }
                ffi::AVMediaType_AVMEDIA_TYPE_AUDIO => {
                    println!(
                        "Audio Codec: {} channels, sample rate {}",
                        local_codec_params.channels, local_codec_params.sample_rate
                    );
                }
                _ => {}
            };

            let codec_name = unsafe { CStr::from_ptr(local_codec.name) }
                .to_str()
                .unwrap();

            println!(
                "\tCodec {} ID {} bit_rate {}",
                codec_name, local_codec.id, local_codec_params.bit_rate
            );
        }
        let (width, height) = resolution.expect("No resolution found");

        let mut codec_context = OwnedAvCodecContext::new(codec_ptr);

        if unsafe {
            ffi::avcodec_parameters_to_context(codec_context.deref_mut(), codec_parameters_ptr)
        } < 0
        {
            panic!("failed to copy codec params to codec context");
        }

        if unsafe { ffi::avcodec_open2(codec_context.deref_mut(), codec_ptr, ptr::null_mut()) } < 0
        {
            panic!("failed to open codec through avcodec_open2");
        }

        let frame = OwnedAvFrame::new();
        let packet = OwnedAvPacket::new();

        let mut packets_waiting = 8;

        RawFrameIter {
            frame_context: format_context,
            packet,
            frame,
            codec_context,
        }
    }
}

impl RawFrameIter {
    fn next(&mut self) -> Option<&OwnedAvFrame> {
        if unsafe { ffi::av_read_frame(format_context, packet) } >= 0 {
            if video_stream_index == Some(packet.stream_index as usize) {
                println!("AVPacket->pts {}", packet.pts);
                decode_packet(packet, codec_context, frame).unwrap();
                packets_waiting -= 1;
                if packets_waiting <= 0 {
                    break;
                }
            }
            unsafe { ffi::av_packet_unref(packet) };
            Some(&self.frame)
        } else {
            None
        }
    }
}

macro_rules! owned_ptr {
    ($name:ident, $ptr:path) => {
        struct $name {
            ptr: NonNull<$ptr>,
        }

        impl Deref for $name {
            type Target = $ptr;

            fn deref(&self) -> &Self::Target {
                unsafe { self.ptr.as_ref() }
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { self.ptr.as_mut() }
            }
        }
    };

    ($name:ident, $ptr:path, $alloc:path, $free:path) => {
        owned_ptr!($name, $ptr);

        impl $name {
            pub fn new() -> Self {
                let ptr = NonNull::new(unsafe { $alloc() })
                    .expect("failed to allocated memory for $name");
                $name { ptr }
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe {
                    $free(&mut (self.ptr.as_ptr()));
                }
            }
        }
    };
}

owned_ptr!(
    OwnedAvFrame,
    AVFrame,
    ffi::av_frame_alloc,
    ffi::av_frame_free
);
owned_ptr!(
    OwnedAvPacket,
    AVPacket,
    ffi::av_packet_alloc,
    ffi::av_packet_free
);
owned_ptr!(
    OwnedAvFormatContext,
    AVFormatContext,
    ffi::avformat_alloc_context,
    ffi::avformat_close_input
);

owned_ptr!(OwnedAvCodecContext, AVCodecContext);
impl OwnedAvCodecContext {
    pub fn new(codec: *const ffi::AVCodec) -> Self {
        let ptr = NonNull::new(unsafe { ffi::avcodec_alloc_context3(codec) })
            .expect("failed to allocated memory for $name");
        OwnedAvCodecContext { ptr }
    }
}

impl Drop for OwnedAvCodecContext {
    fn drop(&mut self) {
        unsafe {
            ffi::avcodec_free_context(&mut (self.ptr.as_ptr()));
        }
    }
}
