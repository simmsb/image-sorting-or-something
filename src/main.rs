use image;
use ndarray::prelude::*;
use std::collections::HashMap;

mod array_sort;

type Rgb8 = image::Rgb<u8>;
type ImageRGB8 = image::ImageBuffer<Rgb8, Vec<u8>>;

const IMG_LEN: usize = 256;

struct ColourMapElem {
    idx: u32,
    pixels: Vec<Rgb8>,
}

impl ColourMapElem {
    fn new() -> Self {
        ColourMapElem {
            idx: 0,
            pixels: vec![],
        }
    }
}

struct ColourMap {
    map: HashMap<Rgb8, ColourMapElem>,
}

impl ColourMap {
    fn fetch_replacement(&mut self, pix: Rgb8) -> Rgb8 {
        let elem = self.map.get_mut(&pix).unwrap();
        let res_pix = elem.pixels[elem.idx as usize];

        elem.idx += 1;

        res_pix
    }

    fn load_from_image(from: &ImageRGB8, to: &ImageRGB8) -> ColourMap {
        let mut map: HashMap<Rgb8, ColourMapElem> =
            from.pixels().map(|p| (*p, ColourMapElem::new())).collect();

        for (f_pix, t_pix) in from.pixels().zip(to.pixels()) {
            map.get_mut(f_pix).unwrap().pixels.push(*t_pix);
        }

        ColourMap { map }
    }

    fn reset(&mut self) {
        for v in self.map.values_mut() {
            v.idx = 0;
        }
    }
}

fn get_factors(num: usize) -> impl Iterator<Item = (usize, usize)> {
    (1..=num)
        .filter(move |i| num % i == 0)
        .map(move |i| (num / i, i))
}

fn construct_sorted_frame(frame: Array2<Rgb8>, shape: (usize, usize)) -> Array2<Rgb8> {
    use array_sort::{PermuteArray, SortArray};
    // what?

    let frame = frame.into_shape(shape).unwrap();
    let perm = frame.sort_axis_by(Axis(1), |i, j| frame[[0, i]].data > frame[[0, j]].data);
    let frame = frame.permute_axis(Axis(1), &perm);
    frame
}

fn generate_frames(
    mut colour_map: ColourMap,
    mut img0: Array2<Rgb8>,
    mut img1: Array2<Rgb8>,
) -> Vec<Array2<Rgb8>> {
    let mut frames = vec![];
    let mut way_back = vec![];

    for (x, y) in get_factors(IMG_LEN * IMG_LEN) {
        img0 = construct_sorted_frame(img0, (x, y));
        img1 = construct_sorted_frame(img1, (x, y));

        frames.push(img0.clone().into_shape((IMG_LEN, IMG_LEN)).unwrap());
        way_back.push(img1.clone().into_shape((IMG_LEN, IMG_LEN)).unwrap());
    }

    for mut arr in way_back.into_iter().rev() {
        arr.map_inplace(|p| {
            *p = colour_map.fetch_replacement(*p);
        });

        frames.push(arr);

        colour_map.reset();
    }

    for _ in 0..5 {
        frames.insert(0, frames[0].clone());
        frames.push(frames.last().unwrap().clone());
    }

    frames.extend(frames.clone().into_iter().rev());

    frames
}

fn main() {
    let img0 = image::open("smugphos.png")
        .unwrap()
        .resize_exact(IMG_LEN as u32, IMG_LEN as u32, image::FilterType::Nearest)
        .to_rgb();
    let img1 = image::open("gnome.jpg")
        .unwrap()
        .resize_exact(IMG_LEN as u32, IMG_LEN as u32, image::FilterType::Nearest)
        .to_rgb();

    let colour_map = ColourMap::load_from_image(&img1, &img0);

    let img0 = Array::from_shape_vec(
        (IMG_LEN, IMG_LEN),
        img0.pixels().cloned().collect::<Vec<_>>(),
    )
    .unwrap();
    let img1 = Array::from_shape_vec(
        (IMG_LEN, IMG_LEN),
        img1.pixels().cloned().collect::<Vec<_>>(),
    )
    .unwrap();

    let frames = generate_frames(colour_map, img0, img1);

    let output_file = std::fs::File::create("transform.gif").unwrap();
    let mut encoder = image::gif::Encoder::new(output_file);

    println!("{} frames", frames.len());

    for frame in frames {
        let pixels: Vec<_> = frame.iter().flat_map(|p| p.data.iter().cloned()).collect();
        let frame = image::gif::Frame::from_rgb_speed(IMG_LEN as u16, IMG_LEN as u16, &pixels, 30);
        encoder.encode(&frame).unwrap();
    }
}
