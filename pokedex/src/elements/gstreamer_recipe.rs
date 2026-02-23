use iced::futures::Stream;
use iced::futures::channel::mpsc::Sender;
use iced::futures::SinkExt;
use iced::Subscription;

use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use gst::prelude::*;

/// A raw RGBA frame from GStreamer
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// The iced Recipe that streams GStreamer frames
pub struct GStreamerRecipe {
    pipeline_description: String,
}

impl GStreamerRecipe {
    pub fn new(pipeline_description: impl Into<String>) -> Self {
        Self {
            pipeline_description: pipeline_description.into(),
        }
    }
}

pub fn gstreamer_stream() -> impl Stream<Item = VideoFrame> {
    iced::stream::channel(8, |mut tx: Sender<VideoFrame>| async move {
        const PIPELINE: &str = "libcamerasrc ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink sync=false";
        
        let (gst_tx, mut gst_rx) = tokio::sync::mpsc::channel::<VideoFrame>(8);

        std::thread::spawn(move || {
            gst::init().unwrap();

            let pipeline = gst::parse::launch(PIPELINE)
                .unwrap()
                .downcast::<gst::Pipeline>()
                .unwrap();

            let appsink = pipeline
                .by_name("sink")
                .unwrap()
                .downcast::<gst_app::AppSink>()
                .unwrap();

            appsink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |sink| {
                        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
                        let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                        let info = gst_video::VideoInfo::from_caps(caps)
                            .map_err(|_| gst::FlowError::Error)?;
                        let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                        let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                        let _ = gst_tx.try_send(VideoFrame {
                            width: info.width(),
                            height: info.height(),
                            data: map.as_slice().to_vec(),
                        });
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );

            pipeline.set_state(gst::State::Playing).unwrap();

            let bus = pipeline.bus().unwrap();
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Eos(..) => break,
                    MessageView::Error(err) => {
                        eprintln!("GStreamer error: {}", err.error());
                        break;
                    }
                    _ => {}
                }
            }
            pipeline.set_state(gst::State::Null).ok();
        });

        while let Some(frame) = gst_rx.recv().await {
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    })
}