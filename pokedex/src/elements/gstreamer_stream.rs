use log::error;

use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::channel::mpsc::Sender;

use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use tokio::sync::watch;

#[derive(Debug, Clone)]
pub enum VideoError {
    PipelineError(String),
    Eos,
}

// A raw RGBA frame from GStreamer
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

fn init_pipeline(pipeline: &str) -> Result<gst::Pipeline, VideoError> {
    let pipeline = gst::parse::launch(pipeline)
        .map_err(|e| VideoError::PipelineError(e.to_string()))?
        .downcast::<gst::Pipeline>()
        .map_err(|_| VideoError::PipelineError("Failed to downcast pipeline".to_string()))?;

    Ok(pipeline)
}

/**
 * Creates GStreamer pipeline
 * Moves it into its own thread
 * Create iced mpsc channel
 * Send gstreamer frames into the channel
 * Frames consumed by iced subscription
 */
pub fn gstreamer_stream() -> impl Stream<Item = Result<VideoFrame, VideoError>> {
    iced::stream::channel(
        1,
        |mut tx: Sender<Result<VideoFrame, VideoError>>| async move {
            // Modern RPI uses libcamerasrc, older ones used v4l2

            #[cfg(target_os = "linux")]
            const PIPELINE_SRC: &str = "libcamerasrc";

            #[cfg(target_os = "windows")]
            const PIPELINE_SRC: &str = "mfvideosrc";

            // then build the full string at runtime
            let pipeline = format!(
                "{} ! videoconvert ! videoscale ! video/x-raw,format=RGBA,width=640,height=480,framerate=30/1 ! queue max-size-buffers=1 leaky=downstream ! appsink name=sink sync=false max-lateness=1",
                PIPELINE_SRC
            );

            // Only holds the latest frame
            let (gst_tx, mut gst_rx) =
                watch::channel::<Option<Result<VideoFrame, VideoError>>>(None);

            std::thread::spawn(move || {
                gst::init().unwrap();

                let pipeline = match init_pipeline(&pipeline) {
                    Ok(pipeline) => pipeline,
                    Err(vid_err) => {
                        let _ = gst_tx.send(Some(Err(vid_err)));
                        panic!("Failed to start pipeline.")
                    }
                };

                let appsink = pipeline
                    .by_name("sink")
                    .unwrap()
                    .downcast::<gst_app::AppSink>()
                    .unwrap();

                // clone transmitter here
                let gst_tx_for_appsink = gst_tx.clone();

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

                            let _ = gst_tx_for_appsink.send(Some(Ok(VideoFrame {
                                width: info.width(),
                                height: info.height(),
                                data: map.as_slice().to_vec(),
                            })));
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
                        MessageView::Eos(..) => {
                            // so it can be moved here
                            let _ = gst_tx.send(Some(Err(VideoError::Eos)));
                            break;
                        }
                        MessageView::Error(err) => {
                            error!("GStreamer error: {}", err.error());
                            let _ = gst_tx.send(Some(Err(VideoError::PipelineError(
                                err.error().to_string(),
                            ))));
                            break;
                        }
                        _ => {}
                    }
                }
                pipeline.set_state(gst::State::Null).ok();
            });

            // Push to channel
            loop {
                gst_rx.changed().await.ok();
                let value = gst_rx.borrow().clone();
                if let Some(result) = value {
                    if tx.send(result).await.is_err() {
                        error!("Failed to send frame over GStreamer channel");
                        break;
                    }
                }
            }
        },
    )
}
