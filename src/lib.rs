#![feature(test)]

#[cfg(test)]
extern crate png;
#[cfg(test)]
extern crate test;

trait Pixel {
    fn red_f32(&self) -> f32;
    fn green_f32(&self) -> f32;
    fn blue_f32(&self) -> f32;
    fn red_u8(&self) -> u8;
    fn green_u8(&self) -> u8;
    fn blue_u8(&self) -> u8;
}

fn color_f32_to_u32(r: f32, g: f32, b: f32) -> u32 {
    color_u8_to_u32(
        (r as u32 & 0xFF) as u8,
        (g as u32 & 0xFF) as u8,
        (b as u32 & 0xFF) as u8,
    )
}

fn color_u8_to_u32(r: u8, g: u8, b: u8) -> u32 {
    (r as u32 & 0xFF) << 16 | (g as u32 & 0xFF) << 8 | (b as u32 & 0xFF)
}

impl Pixel for u32 {
    fn red_f32(&self) -> f32 {
        self.red_u8() as f32
    }
    fn green_f32(&self) -> f32 {
        self.green_u8() as f32
    }
    fn blue_f32(&self) -> f32 {
        self.blue_u8() as f32
    }
    fn red_u8(&self) -> u8 {
        ((self & 0xFF0000) >> 16) as u8
    }
    fn green_u8(&self) -> u8 {
        ((self & 0x00FF00) >> 8) as u8
    }
    fn blue_u8(&self) -> u8 {
        (self & 0x0000FF) as u8
    }
}

/// Calculates the weighted difference between two pixels.
///
/// These are the steps:
///
/// 1. Finds absolute color diference between two pixels.
/// 2. Converts color difference into Y'UV, seperating color from light.
/// 3. Applies Y'UV thresholds, giving importance to luminance.
fn diff<T: Pixel>(pixel_a: T, pixel_b: T) -> f32 {
    // Weights should emphasize luminance (Y), in order to work. Feel free to experiment.
    const Y_WEIGHT: f32 = 48.0;
    const U_WEIGHT: f32 = 7.0;
    const V_WEIGHT: f32 = 6.0;

    let r = (pixel_a.red_f32() - pixel_b.red_f32()).abs();
    let b = (pixel_a.blue_f32() - pixel_b.blue_f32()).abs();
    let g = (pixel_a.green_f32() - pixel_b.green_f32()).abs();
    let y = r * 0.299000 + g * 0.587000 + b * 0.114000;
    let u = r * -0.168736 + g * -0.331264 + b * 0.500000;
    let v = r * 0.500000 + g * -0.418688 + b * -0.081312;
    let weight = (y * Y_WEIGHT) + (u * U_WEIGHT) + (v * V_WEIGHT);
    weight
}

/// Blends two pixels together and retuns an new Pixel.
fn blend<T: Pixel>(pixel_a: T, pixel_b: T, alpha: f32) -> u32 {
    let reverse_alpha = 1.0 - alpha;

    color_f32_to_u32(
        (alpha * pixel_b.red_f32()) + (reverse_alpha * pixel_a.red_f32()),
        (alpha * pixel_b.green_f32()) + (reverse_alpha * pixel_a.green_f32()),
        (alpha * pixel_b.blue_f32()) + (reverse_alpha * pixel_a.blue_f32()),
    )
}

/// Applies the xBR filter.
pub fn apply(buf: &mut [u32], image: &[u32], width: u32, height: u32) {
    const SCALE: i32 = 2;

    let src_width = width as i32;
    let src_height = height as i32;
    let scaled_width = src_width * SCALE;

    let pixel_at = |x: i32, y: i32| {
        if x < 0 || x >= src_width || y < 0 || y >= src_height {
            0
        } else {
            image[(src_width * y + x) as usize]
        }
    };

    let matrix = &mut [0; 21];

    for y in 0..src_height {
        for x in 0..src_width {
            // Matrix: 10 is (0,0) i.e. current pixel.
            // 	-2 | -1|  0| +1| +2 	(x)
            // ______________________________
            // -2 |	    [ 0][ 1][ 2]
            // -1 |	[ 3][ 4][ 5][ 6][ 7]
            //  0 |	[ 8][ 9][10][11][12]
            // +1 |	[13][14][15][16][17]
            // +2 |	    [18][19][20]
            // (y)|

            matrix[0] = pixel_at(x - 1, y - 2);
            matrix[1] = pixel_at(x, y - 2);
            matrix[2] = pixel_at(x + 1, y - 2);
            matrix[3] = pixel_at(x - 2, y - 1);
            matrix[4] = pixel_at(x - 1, y - 1);
            matrix[5] = pixel_at(x, y - 1);
            matrix[6] = pixel_at(x + 1, y - 1);
            matrix[7] = pixel_at(x + 2, y - 1);
            matrix[8] = pixel_at(x - 2, y);
            matrix[9] = pixel_at(x - 1, y);
            matrix[10] = pixel_at(x, y);
            matrix[11] = pixel_at(x + 1, y);
            matrix[12] = pixel_at(x + 2, y);
            matrix[13] = pixel_at(x - 2, y + 1);
            matrix[14] = pixel_at(x - 1, y + 1);
            matrix[15] = pixel_at(x, y + 1);
            matrix[16] = pixel_at(x + 1, y + 1);
            matrix[17] = pixel_at(x + 2, y + 1);
            matrix[18] = pixel_at(x - 1, y + 2);
            matrix[19] = pixel_at(x, y + 2);
            matrix[20] = pixel_at(x + 1, y + 2);

            // Calculate color weights using 2 points in the matrix
            let d_10_9 = diff(matrix[10], matrix[9]);
            let d_10_5 = diff(matrix[10], matrix[5]);
            let d_10_11 = diff(matrix[10], matrix[11]);
            let d_10_15 = diff(matrix[10], matrix[15]);
            let d_10_14 = diff(matrix[10], matrix[14]);
            let d_10_6 = diff(matrix[10], matrix[6]);
            let d_4_8 = diff(matrix[4], matrix[8]);
            let d_4_1 = diff(matrix[4], matrix[1]);
            let d_9_5 = diff(matrix[9], matrix[5]);
            let d_9_15 = diff(matrix[9], matrix[15]);
            let d_9_3 = diff(matrix[9], matrix[3]);
            let d_5_11 = diff(matrix[5], matrix[11]);
            let d_5_0 = diff(matrix[5], matrix[0]);
            let d_10_4 = diff(matrix[10], matrix[4]);
            let d_10_16 = diff(matrix[10], matrix[16]);
            let d_6_12 = diff(matrix[6], matrix[12]);
            let d_6_1 = diff(matrix[6], matrix[1]);
            let d_11_15 = diff(matrix[11], matrix[15]);
            let d_11_7 = diff(matrix[11], matrix[7]);
            let d_5_2 = diff(matrix[5], matrix[2]);
            let d_14_8 = diff(matrix[14], matrix[8]);
            let d_14_19 = diff(matrix[14], matrix[19]);
            let d_15_18 = diff(matrix[15], matrix[18]);
            let d_9_13 = diff(matrix[9], matrix[13]);
            let d_16_12 = diff(matrix[16], matrix[12]);
            let d_16_19 = diff(matrix[16], matrix[19]);
            let d_15_20 = diff(matrix[15], matrix[20]);
            let d_15_17 = diff(matrix[15], matrix[17]);

            // Top Left Edge Detection Rule
            let a1 = d_10_14 + d_10_6 + d_4_8 + d_4_1 + 4.0 * d_9_5;
            let b1 = d_9_15 + d_9_3 + d_5_11 + d_5_0 + 4.0 * d_10_4;
            let idx = ((y * SCALE) * scaled_width) + (x * SCALE);

            buf[idx as usize] = if a1 < b1 {
                let new_pixel = if d_10_9 <= d_10_5 {
                    matrix[9]
                } else {
                    matrix[5]
                };
                let blended_pixel = blend(new_pixel, matrix[10], 0.5);
                blended_pixel
            } else {
                matrix[10]
            };

            // Top Right Edge Detection Rule
            let a2 = d_10_16 + d_10_4 + d_6_12 + d_6_1 + 4.0 * d_5_11;
            let b2 = d_11_15 + d_11_7 + d_9_5 + d_5_2 + 4.0 * d_10_6;
            let idx = ((y * SCALE) * scaled_width) + (x * SCALE + 1);
            buf[idx as usize] = if a2 < b2 {
                let new_pixel = if d_10_5 <= d_10_11 {
                    matrix[5]
                } else {
                    matrix[11]
                };
                let blended_pixel = blend(new_pixel, matrix[10], 0.5);
                blended_pixel
            } else {
                matrix[10]
            };

            // Bottom Left Edge Detection Rule
            let a3 = d_10_4 + d_10_16 + d_14_8 + d_14_19 + 4.0 * d_9_15;
            let b3 = d_9_5 + d_9_13 + d_11_15 + d_15_18 + 4.0 * d_10_14;
            let idx = ((y * SCALE + 1) * scaled_width) + (x * SCALE);
            buf[idx as usize] = if a3 < b3 {
                let new_pixel = if d_10_9 <= d_10_15 {
                    matrix[9]
                } else {
                    matrix[15]
                };
                let blended_pixel = blend(new_pixel, matrix[10], 0.5);
                blended_pixel
            } else {
                matrix[10]
            };

            // Bottom Right Edge Detection Rule
            let a4 = d_10_6 + d_10_14 + d_16_12 + d_16_19 + 4.0 * d_11_15;
            let b4 = d_9_15 + d_15_20 + d_15_17 + d_5_11 + 4.0 * d_10_16;
            let idx = ((y * SCALE + 1) * scaled_width) + (x * SCALE + 1);
            buf[idx as usize] = if a4 < b4 {
                let new_pixel = if d_10_11 <= d_10_15 {
                    matrix[11]
                } else {
                    matrix[15]
                };
                let blended_pixel = blend(new_pixel, matrix[10], 0.5);
                blended_pixel
            } else {
                matrix[10]
            };
        }
    }
}

pub fn get_buffer_for_size(width: u32, height: u32) -> (Vec<u32>, u32, u32) {
    (
        vec![0; (width as usize) * 2 * (height as usize) * 2],
        width * 2,
        height * 2,
    )
}

#[cfg(test)]
mod tests {

    use super::*;
    use test::Bencher;

    use std::fs::File;
    use std::io::BufReader;
    use std::io::BufWriter;
    use std::path::Path;

    #[bench]
    fn bench_xbr(b: &mut Bencher) {
        let (img, info) = load_img("./assets/input.png").expect("Could not load input image");

        let input: Vec<u32> = match info.color_type {
            png::ColorType::RGB => (0..(info.width * info.height) as usize)
                .map(|i| color_u8_to_u32(img[i * 3 + 0], img[i * 3 + 1], img[i * 3 + 2]))
                .collect(),
            png::ColorType::RGBA => (0..(info.width * info.height) as usize)
                .map(|i| color_u8_to_u32(img[i * 4 + 0], img[i * 4 + 1], img[i * 4 + 2]))
                .collect(),
            _ => unimplemented!(),
        };

        let (mut out_buf, out_width, out_height) = get_buffer_for_size(info.width, info.height);
        b.iter(|| apply(&mut out_buf[..], &input, info.width, info.height));

        save_img("./assets/output.png", out_width, out_height, &out_buf[..])
            .expect("Could not save output image");
    }

    fn load_img(path: &str) -> Result<(Vec<u8>, png::OutputInfo), std::io::Error> {
        let file = File::open(Path::new(path))?;
        let ref mut r = BufReader::new(file);
        let decoder = png::Decoder::new(r);
        let (info, mut reader) = decoder.read_info()?;
        let mut buf = vec![0; info.buffer_size()];

        reader.next_frame(&mut buf)?;

        Ok((buf, info))
    }

    fn explode_rgb(buf: &[u32]) -> Vec<u8> {
        (0..buf.len() * 3)
            .map(|i| ((buf[(i / 3)] >> (8 * (2 - (i % 3)))) & 0xFF) as u8)
            .collect()
    }

    fn save_img(path: &str, width: u32, height: u32, data: &[u32]) -> Result<(), std::io::Error> {
        let file = File::create(Path::new(path))?;
        let ref mut w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, width, height);

        encoder.set_color(png::ColorType::RGB);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Default);
        encoder.set_filter(png::FilterType::NoFilter);

        let mut writer = encoder.write_header()?;

        writer.write_image_data(&explode_rgb(data)[..])?;

        Ok(())
    }
}
