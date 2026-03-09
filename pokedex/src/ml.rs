use ort::value::Tensor;
use ort::{inputs, execution_providers::CoreMLExecutionProvider};
use ort::session::Session;
use anyhow::{Context, Error, Result};

use image::DynamicImage;
use ndarray::Array4;

use std::sync::{Arc, Mutex};


pub fn init(model_path: &str) -> Result<Arc<Mutex<Session>>, Error> {
    // investigate parameters here
    ort::init()
        .with_execution_providers([CoreMLExecutionProvider::default().build()])
        .commit()
        .then_some(())
        .context("Failed to commit to ONNX runtime")?;

    let model = Session::builder()?
        .commit_from_file(model_path)?;

    Ok(Arc::new(Mutex::new(model)))
}

fn image_to_tensor(img: &DynamicImage) -> Array4<f32> {
    // TODO: test with nearest-neighbor
    let img = img.resize_exact(224, 224, image::imageops::FilterType::Triangle)
        .to_rgb8();
    
    let mut tensor: ndarray::ArrayBase<ndarray::OwnedRepr<f32>, ndarray::Dim<[usize; 4]>, f32> = Array4::<f32>::zeros((1, 3, 224, 224));
    for (x, y, pixel) in img.enumerate_pixels() {
        tensor[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
        tensor[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
        tensor[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
    }
    tensor
}

pub fn classify_image(session: &mut Session, img_path: &str) -> Result<(usize, f32), ort::Error> {
    let img = image::open(img_path).unwrap();
    let tensor = image_to_tensor(&img);
    let input = Tensor::from_array(tensor)?;

    let outputs = session.run(inputs!["images" => input])?;
    
    let output: (&ort::value::Shape, &[f32]) = outputs["output0"].try_extract_tensor::<f32>()?;

    let (_, probs) = output;
    
    // find the class with highest confidence
    let (class_idx, confidence) = probs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, &v)| (i, v))
        .unwrap();

    Ok((class_idx, confidence))
}