use color_eyre::{eyre::eyre, Result};
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecID};
use rsmpeg::avutil::AVFrame;

pub struct Encoder {
    context: AVCodecContext,
}

impl Encoder {
    pub fn new(codec_id: AVCodecID, width: i32, height: i32) -> Result<Encoder> {
        let decoder = AVCodec::find_decoder(codec_id)
            .ok_or_else(|| eyre!("No decoder found for codec {}", codec_id))?;

        let mut context = AVCodecContext::new(&decoder);
        context.set_width(width);
        context.set_height(height);
        context.open(None)?;

        Ok(Encoder { context })
    }

    pub fn encode_frame(&mut self, frame: &AVFrame) -> Result<AVFrame> {
        self.context.send_frame(Some(frame))?;
        Ok(self.context.receive_frame()?)
    }
}
