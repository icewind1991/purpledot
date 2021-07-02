//! Port from Original code: https://github.com/leandromoreira/ffmpeg-libav-tutorial/blob/master/0_hello_world.c

use color_eyre::{eyre::eyre, Report, Result};
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecID, AVCodecParametersRef, AVPacket};
use rsmpeg::avformat::{AVFormatContextInput, AVStreamRef};
use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi;

pub trait FormatContextInputExt {
    fn into_packets(self) -> PacketIterator;
    fn into_frames(self) -> Result<VideoFrames>;
    fn video_stream(&self) -> Result<(i32, AVStreamRef)>;
    fn video_info(&self) -> Result<VideoStreamInfo>;
}

#[derive(Clone, Copy, Debug)]
pub struct VideoStreamInfo {
    pub width: i32,
    pub height: i32,
    pub index: i32,
    pub codec: AVCodecID,
}

impl VideoStreamInfo {
    pub fn new(index: i32, codec_params: &AVCodecParametersRef) -> Self {
        let width = codec_params.width;
        let height = codec_params.height;

        VideoStreamInfo {
            width,
            height,
            index: index as i32,
            codec: codec_params.codec_id,
        }
    }
}

pub struct PacketIterator {
    context: AVFormatContextInput,
}

impl Iterator for PacketIterator {
    type Item = Result<AVPacket>;

    fn next(&mut self) -> Option<Result<AVPacket>> {
        self.context.read_packet().map_err(Report::from).transpose()
    }
}

impl FormatContextInputExt for AVFormatContextInput {
    fn into_packets(self) -> PacketIterator {
        PacketIterator { context: self }
    }

    fn into_frames(self) -> Result<VideoFrames> {
        let (codec_context, info) = {
            let (index, stream) = self.video_stream()?;
            let codec_params = stream.codecpar();
            let info = VideoStreamInfo::new(index, &codec_params);

            let decoder = AVCodec::find_decoder(codec_params.codec_id)
                .ok_or_else(|| eyre!("No decoder found for codec {}", info.codec))?;

            let mut codec_context = AVCodecContext::new(&decoder);
            codec_context.apply_codecpar(codec_params)?;
            codec_context.open(None)?;
            (codec_context, info)
        };
        Ok(VideoFrames {
            packets: self.into_packets(),
            codec_context,
            info,
        })
    }

    fn video_stream(&self) -> Result<(i32, AVStreamRef)> {
        self.streams()
            .into_iter()
            .enumerate()
            .find(|(_index, stream)| {
                stream.codecpar().codec_type == ffi::AVMediaType_AVMEDIA_TYPE_VIDEO
            })
            .map(|(index, stream)| (index as i32, stream))
            .ok_or_else(|| eyre!("No video stream found"))
    }

    fn video_info(&self) -> Result<VideoStreamInfo> {
        let (index, stream) = self.video_stream()?;
        let codec_params = stream.codecpar();
        Ok(VideoStreamInfo::new(index, &codec_params))
    }
}

pub struct VideoFrames {
    packets: PacketIterator,
    codec_context: AVCodecContext,
    pub info: VideoStreamInfo,
}

impl Iterator for VideoFrames {
    type Item = Result<AVFrame>;

    fn next(&mut self) -> Option<Result<AVFrame>> {
        self.packets
            .next()
            .filter(|res| match res {
                Ok(packet) => packet.stream_index == self.info.index,
                Err(_) => true,
            })
            .map(|res| {
                res.and_then(|packet| {
                    self.codec_context.send_packet(Some(&packet))?;
                    Ok(self.codec_context.receive_frame()?)
                })
            })
    }
}
