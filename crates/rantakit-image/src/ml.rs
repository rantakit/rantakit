//! Learned paper finding and cleaning, experimental: behind the `ml`
//! feature, off by default and not in release builds.
//!
//! - DocCornerNet finds the paper's four corners
//!   (<https://github.com/mapo-t2i/DocCornerNet-CoordClass>, MIT).
//! - GCDRNet removes shadows and whitens the paper
//!   (<https://github.com/ZZZHANG-jx/GCDRNet>, MIT; ONNX from
//!   <https://github.com/sdcb/SimdGcdrNet>, Apache-2.0).
//!
//! Both run on tract, which is pure Rust. The model files are not in git:
//! `models/README.md` says where they come from. Every function returns
//! None when a model fails, so the caller falls back to the classic code.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

use ::image::imageops::{FilterType, resize};
use ::image::{DynamicImage, RgbImage};
use tract_onnx::prelude::*;

use crate::paper::Corners;

static DOC_CORNER_NET: &[u8] = include_bytes!("../models/doccornernet.onnx");
static GC_NET: &[u8] = include_bytes!("../models/gcdrnet_gcnet.onnx");
static DR_NET: &[u8] = include_bytes!("../models/gcdrnet_drnet.onnx");

type Plan = Arc<TypedSimplePlan>;
/// GC-Net and DR-Net plans by padded (height, width).
type GcdrPlans = Mutex<HashMap<(usize, usize), (Plan, Plan)>>;

/// DocCornerNet's input side; the photo is stretched to a square this size.
const CORNER_SIDE: usize = 224;
/// ImageNet normalization, as DocCornerNet was trained.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];
/// Below this score DocCornerNet saw no paper.
const MIN_SCORE: f32 = 0.5;
/// A smaller quad, as a share of the photo, is more likely a mistake.
const MIN_AREA: f32 = 0.05;

/// GCDRNet wants sides that are a multiple of this.
const STRIDE: usize = 32;
/// GCDRNet's cost grows with the pixel count, so a larger page is made this
/// long first. About 190 dpi on an A4 page.
const CLEAN_SIDE: u32 = 1600;
/// Plans are built per input shape; a few are kept for previews and pages.
const KEPT_PLANS: usize = 4;

/// An ONNX model parsed once; plans for each input shape are made from it.
fn parsed(cell: &'static OnceLock<Option<InferenceModel>>, bytes: &[u8]) -> Option<&'static InferenceModel> {
    cell.get_or_init(|| {
        tract_onnx::onnx()
            // The exported files carry shape hints that tract cannot unify
            // with a fixed input shape; tract works the shapes out itself.
            .with_ignore_output_shapes(true)
            .with_ignore_value_info(true)
            .model_for_read(&mut Cursor::new(bytes))
            .ok()
    })
    .as_ref()
}

fn plan_for(model: &InferenceModel, shape: &[usize]) -> TractResult<Plan> {
    model
        .clone()
        .with_input_fact(0, f32::fact(shape).into())?
        .into_optimized()?
        .into_runnable()
}

/// The paper's corners by DocCornerNet, or None when it sees no paper or
/// its corners make no sensible page.
pub fn find_paper(img: &DynamicImage) -> Option<Corners> {
    static MODEL: OnceLock<Option<InferenceModel>> = OnceLock::new();
    static PLAN: OnceLock<Option<Plan>> = OnceLock::new();
    let plan = PLAN
        .get_or_init(|| plan_for(parsed(&MODEL, DOC_CORNER_NET)?, &[1, CORNER_SIDE, CORNER_SIDE, 3]).ok())
        .as_ref()?;

    // A thumbnail first keeps the stretch quick on a large photo.
    let side = CORNER_SIDE as u32;
    let small = img.thumbnail(side * 2, side * 2).to_rgb8();
    let small = resize(&small, side, side, FilterType::Triangle);
    let input = tract_ndarray::Array4::from_shape_fn((1, CORNER_SIDE, CORNER_SIDE, 3), |(_, y, x, c)| {
        (small.get_pixel(x as u32, y as u32)[c] as f32 / 255.0 - MEAN[c]) / STD[c]
    });
    let outputs = plan.run(tvec!(input.into_tensor().into())).ok()?;
    let coords: Vec<f32> = outputs[0].to_plain_array_view::<f32>().ok()?.iter().copied().collect();
    let logit = *outputs[1].to_plain_array_view::<f32>().ok()?.iter().next()?;
    if 1.0 / (1.0 + (-logit).exp()) < MIN_SCORE || coords.len() != 8 {
        return None;
    }
    let corners: Corners = std::array::from_fn(|i| [coords[2 * i].clamp(0.0, 1.0), coords[2 * i + 1].clamp(0.0, 1.0)]);
    is_page(&corners).then_some(corners)
}

/// Whether the corners, clockwise from the top left, make a convex
/// four-sided shape of a sensible size. A crossed or folded quad would
/// straighten into a garbled page.
fn is_page(c: &Corners) -> bool {
    let turns = (0..4).map(|i| {
        let (a, b, d) = (c[i], c[(i + 1) % 4], c[(i + 2) % 4]);
        (b[0] - a[0]) * (d[1] - b[1]) - (b[1] - a[1]) * (d[0] - b[0])
    });
    // Clockwise on screen (y down) turns the same way at every corner.
    let convex = turns.into_iter().all(|t| t > 0.0);
    let area = 0.5 * (0..4).map(|i| c[i][0] * c[(i + 1) % 4][1] - c[(i + 1) % 4][0] * c[i][1]).sum::<f32>();
    convex && area >= MIN_AREA
}

/// `img` with shadows removed and its paper whitened by GCDRNet, at most
/// `CLEAN_SIDE` pixels on its longest side; None when the model fails.
pub fn cleaned(img: &RgbImage) -> Option<RgbImage> {
    static GC_MODEL: OnceLock<Option<InferenceModel>> = OnceLock::new();
    static DR_MODEL: OnceLock<Option<InferenceModel>> = OnceLock::new();
    static PLANS: OnceLock<GcdrPlans> = OnceLock::new();

    let img = fit_within(img, CLEAN_SIDE);
    let (width, height) = (img.width() as usize, img.height() as usize);
    if width < 2 || height < 2 {
        return None;
    }
    let (pad_y, pad_x) = ((STRIDE - height % STRIDE) % STRIDE, (STRIDE - width % STRIDE) % STRIDE);
    let (h, w) = (height + pad_y, width + pad_x);

    let (gc, dr) = {
        let mut plans = PLANS.get_or_init(Default::default).lock().ok()?;
        match plans.get(&(h, w)) {
            Some(pair) => pair.clone(),
            None => {
                let gc = plan_for(parsed(&GC_MODEL, GC_NET)?, &[1, 3, h, w]).ok()?;
                let dr = plan_for(parsed(&DR_MODEL, DR_NET)?, &[1, 6, h, w]).ok()?;
                if plans.len() >= KEPT_PLANS {
                    plans.clear();
                }
                plans.insert((h, w), (gc.clone(), dr.clone()));
                (gc, dr)
            }
        }
    };

    // BGR (GCDRNet was trained on OpenCV images), 0..1, padded at the top
    // and left by repeating the edge.
    let input = tract_ndarray::Array4::from_shape_fn((1, 3, h, w), |(_, c, y, x)| {
        img.get_pixel(x.saturating_sub(pad_x) as u32, y.saturating_sub(pad_y) as u32)[2 - c] as f32 / 255.0
    });
    let shading = gc.run(tvec!(input.clone().into_tensor().into())).ok()?.remove(0);
    let shading = shading.to_plain_array_view::<f32>().ok()?.into_dimensionality::<tract_ndarray::Ix4>().ok()?;
    let divided = tract_ndarray::Zip::from(&input)
        .and(&shading)
        .map_collect(|x, s| (x / s.max(1e-6)).clamp(0.0, 1.0));
    let both = tract_ndarray::concatenate(tract_ndarray::Axis(1), &[input.view(), divided.view()]).ok()?;
    let output = dr.run(tvec!(both.into_tensor().into())).ok()?.remove(0);
    let output = output.to_plain_array_view::<f32>().ok()?;

    Some(RgbImage::from_fn(width as u32, height as u32, |x, y| {
        let (x, y) = (x as usize + pad_x, y as usize + pad_y);
        ::image::Rgb(std::array::from_fn(|c| (output[[0, 2 - c, y, x]].clamp(0.0, 1.0) * 255.0) as u8))
    }))
}

fn fit_within(img: &RgbImage, max_side: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= max_side {
        return img.clone();
    }
    let scale = max_side as f32 / w.max(h) as f32;
    let (sw, sh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
    resize(img, sw, sh, FilterType::Triangle)
}
