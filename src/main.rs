use clap::Parser;
use image::Rgb;
use imageproc::rect::Rect;
use ocrs::{ImageSource, OcrEngineParams};
use pdfium_render::prelude::{PdfRenderConfig, Pdfium};
use regex::Regex;
use rten::Model;
use rten_imageproc::BoundingRect;
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Args {
    /// Path to the original pdf file
    #[arg(short, long)]
    pdf_path: PathBuf,

    /// Output folder for redacted pictures
    #[arg(short, long)]
    output_folder: PathBuf,

    /// Only run from this page
    #[arg(long, default_value_t = 1)]
    from_page: usize,

    /// Regex pattern for redaction
    #[arg(short, long)]
    redact_pattern: String,

    #[arg(long, default_value_t = 2)]
    x_offset: u32,

    #[arg(long, default_value_t = 2)]
    y_offset: u32,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let detection_model_path = Path::new("text-detection.rten");
    let rec_model_path = Path::new("text-recognition.rten");

    let detection_model = Model::load_file(detection_model_path)?;
    let recognition_model = Model::load_file(rec_model_path)?;

    let engine = ocrs::OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;

    let pdfium = Pdfium::default();
    let render_config = PdfRenderConfig::new()
        .set_maximum_width(2000)
        .set_maximum_height(2000);

    let document = pdfium.load_pdf_from_file(&args.pdf_path, None)?;

    let black = Rgb([0, 0, 0]);
    let redact_pattern = Regex::new(&args.redact_pattern)?;
    std::fs::create_dir_all(&args.output_folder)?;

    for (i, page) in document.pages().iter().enumerate().skip(args.from_page - 1) {
        let mut img = page
            .render_with_config(&render_config)?
            .as_image()
            .into_rgb8();
        let img_source = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
        let ocr_input = engine.prepare_input(img_source)?;
        let word_rects = engine.detect_words(&ocr_input)?;
        let mut censored_word_count = 0usize;
        for word in word_rects {
            let texts = engine.recognize_text(&ocr_input, &[vec![word]])?;
            if !texts
                .iter()
                .flatten()
                .map(|s| s.to_string())
                .any(|s| redact_pattern.is_match(&s))
            {
                continue;
            }
            let corner = word.bounding_rect().top_left();
            imageproc::drawing::draw_filled_rect_mut(
                &mut img,
                Rect::at(corner.x as i32 - args.x_offset as i32, corner.y as i32 - args.y_offset as i32)
                    .of_size(word.width() as u32 + args.x_offset * 2, word.height() as u32 + args.y_offset * 2),
                black,
            );
            censored_word_count += 1;
        }
        img.save(format!("{}/redacted-{i}.png", args.output_folder.display()))?;
        tracing::info!(censored_word_count, page = i, "Written image for a page");
    }
    Ok(())
}
