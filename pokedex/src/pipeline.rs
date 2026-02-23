use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::sync::{mpsc};

pub fn init_pipeline(tx: mpsc::Sender<Vec<u8>>) {
    gst::init().unwrap();

    let pipeline = gst::parse::launch(
        "libcamerasrc ! video/x-raw,format=RGB,width=640,height=480 ! appsink name=sink"
    ).unwrap();

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("Expected pipeline");

    let appsink = pipeline
        .by_name("sink")
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();

    appsink.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("format", "RGB")
            .build(),
    ));

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().unwrap();
                let buffer = sample.buffer().unwrap();
                let map = buffer.map_readable().unwrap();
                let data = map.as_slice(); // raw RGB pixel bytes

                // → send frame to your iced state via channel
                tx.send(Vec::from(data)).unwrap();

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    pipeline.set_state(gst::State::Playing).unwrap();
    // loop {} // or hook into tokio/iced loop

    // pipeline.set_state(gst::State::Null)?;  // cleanup
}
