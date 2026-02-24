use iced::futures::Stream;
use iced::futures::channel::mpsc::Sender;
use iced::futures::SinkExt;

use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use gst::prelude::*;

// A raw RGBA frame from GStreamer
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/**
 * Creates GStreamer pipeline
 * Moves it into its own thread
 * Create iced mpsc channel
 * Send gstreamer frames into the channel
 * Frames consumed by iced subscription
 */
pub fn gstreamer_stream() -> impl Stream<Item = VideoFrame> {
    iced::stream::channel(8, |mut tx: Sender<VideoFrame>| async move {
        // Modern RPI uses libcamerasrc, older ones used v4l2
        const PIPELINE: &str = "libcamerasrc ! videoconvert ! videoscale ! video/x-raw,format=RGBA,width=640,height=480 ! appsink name=sink sync=false";
        
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

            // Convert frames to VideoFrames as they come in
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

            // Handle GStreamer messages
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

        // Push to channel
        while let Some(frame) = gst_rx.recv().await {
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    })
}