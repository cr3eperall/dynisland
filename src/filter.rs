use std::cmp::min;

use anyhow::{Context, Result};

/// works with 4 byte colors (RGBA/ARGB)
pub fn apply_blur(
    surface: &mut gdk::cairo::ImageSurface,
    radius: f32,
    n: usize,
) -> Result<gdk::cairo::ImageSurface> {
    //TODO optimize (maybe remove some iter.collect but i'm too lazy to do it)
    //TODO ultimate optimization: use wgpu
    if radius <= 0.0 {
        return Ok(surface.clone());
    }

    let (width, height) = (surface.width(), surface.height());

    let mut blurred_surface = gdk::cairo::ImageSurface::create(surface.format(), width, height)
        .with_context(|| "failed to create new blur imagesurface")?;
    let mut blurred_surface_data = blurred_surface
        .data()
        .with_context(|| "failed to get raw data from tmp blur surface")?;
    let mut blurred_surface_data2 = blurred_surface_data
        .chunks_exact_mut(4)
        .map(|val| val.try_into().unwrap())
        .collect::<Vec<&mut [u8; 4]>>();

    let surface_data = surface
        .data()
        .with_context(|| "failed to get raw data from tmp surface")?;
    let mut surface_data = surface_data
        .chunks_exact(4)
        .map(|val| val.try_into().unwrap())
        .collect::<Vec<[u8; 4]>>();

    if radius < height as f32 && radius < width as f32 {
        gaussian_blur(
            &mut surface_data,
            width as usize,
            height as usize,
            radius,
            n,
        );
    }

    for i in 0..surface_data.len() {
        blurred_surface_data2[i][0] = surface_data[i][0];
        blurred_surface_data2[i][1] = surface_data[i][1];
        blurred_surface_data2[i][2] = surface_data[i][2];
        blurred_surface_data2[i][3] = surface_data[i][3];
    }
    drop(blurred_surface_data);
    blurred_surface.mark_dirty();
    Ok(blurred_surface)
}

// code borrowed from https://github.com/fschutt/fastblur/blob/master/src/blur.rs
// changed [u8; 3] to [u8; 4] and removed round() for performance
pub fn gaussian_blur(
    data: &mut [[u8; 4]],
    width: usize,
    height: usize,
    blur_radius: f32,
    n: usize,
) {
    let boxes = create_box_gauss(blur_radius, n);
    let mut backbuf = data.to_owned();

    for box_size in boxes.iter() {
        let radius = ((box_size - 1) / 2) as usize;
        box_blur(&mut backbuf, data, width, height, radius, radius);
    }
}

/// Same as gaussian_blur, but allows using different blur radii for vertical and horizontal passes
pub fn gaussian_blur_asymmetric(
    data: &mut [[u8; 4]],
    width: usize,
    height: usize,
    blur_radius_horizontal: f32,
    blur_radius_vertical: f32,
) {
    let boxes_horz = create_box_gauss(blur_radius_horizontal, 3);
    let boxes_vert = create_box_gauss(blur_radius_vertical, 3);
    let mut backbuf = data.to_owned();

    for (box_size_horz, box_size_vert) in boxes_horz.iter().zip(boxes_vert.iter()) {
        let radius_horz = ((box_size_horz - 1) / 2) as usize;
        let radius_vert = ((box_size_vert - 1) / 2) as usize;
        box_blur(&mut backbuf, data, width, height, radius_horz, radius_vert);
    }
}

#[inline]
/// If there is no valid size (e.g. radius is negative), returns `vec![1; len]`
/// which would translate to blur radius of 0
fn create_box_gauss(sigma: f32, n: usize) -> Vec<i32> {
    if sigma > 0.0 {
        let n_float = n as f32;

        // Ideal averaging filter width
        let w_ideal = (12.0 * sigma * sigma / n_float).sqrt() + 1.0;
        let mut wl: i32 = w_ideal.floor() as i32;

        if wl % 2 == 0 {
            wl -= 1;
        };

        let wu = wl + 2;

        let wl_float = wl as f32;
        let m_ideal = (12.0 * sigma * sigma
            - n_float * wl_float * wl_float
            - 4.0 * n_float * wl_float
            - 3.0 * n_float)
            / (-4.0 * wl_float - 4.0);
        let m: usize = m_ideal.round() as usize;

        let mut sizes = Vec::<i32>::new();

        for i in 0..n {
            if i < m {
                sizes.push(wl);
            } else {
                sizes.push(wu);
            }
        }

        sizes
    } else {
        vec![1; n]
    }
}

/// Same as gaussian_blur, but allows using different blur radii for vertical and horizontal passes
pub fn gaussian_blur_asymmetric_single_channel(
    data: &mut [u8],
    width: usize,
    height: usize,
    blur_radius_horizontal: f32,
    blur_radius_vertical: f32,
) {
    let boxes_horz = create_box_gauss(blur_radius_horizontal, 3);
    let boxes_vert = create_box_gauss(blur_radius_vertical, 3);
    let mut backbuf = data.to_owned();

    for (box_size_horz, box_size_vert) in boxes_horz.iter().zip(boxes_vert.iter()) {
        let radius_horz = ((box_size_horz - 1) / 2) as usize;
        let radius_vert = ((box_size_vert - 1) / 2) as usize;
        box_blur_single_channel(&mut backbuf, data, width, height, radius_horz, radius_vert);
    }
}

/// Needs 2x the same image
#[inline]
fn box_blur(
    backbuf: &mut [[u8; 4]],
    frontbuf: &mut [[u8; 4]],
    width: usize,
    height: usize,
    blur_radius_horz: usize,
    blur_radius_vert: usize,
) {
    box_blur_horz(backbuf, frontbuf, width, height, blur_radius_horz);
    box_blur_vert(frontbuf, backbuf, width, height, blur_radius_vert);
}

#[inline]
fn box_blur_vert(
    backbuf: &[[u8; 4]],
    frontbuf: &mut [[u8; 4]],
    width: usize,
    height: usize,
    blur_radius: usize,
) {
    if blur_radius == 0 {
        frontbuf.copy_from_slice(backbuf);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..width {
        let col_start = i; //inclusive
        let col_end = i + width * (height - 1); //inclusive
        let mut ti: usize = i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius * width;

        let fv: [u8; 4] = backbuf[col_start];
        let lv: [u8; 4] = backbuf[col_end];

        let mut val_r: isize = (blur_radius as isize + 1) * isize::from(fv[0]);
        let mut val_g: isize = (blur_radius as isize + 1) * isize::from(fv[1]);
        let mut val_b: isize = (blur_radius as isize + 1) * isize::from(fv[2]);
        let mut val_a: isize = (blur_radius as isize + 1) * isize::from(fv[3]);

        // Get the pixel at the specified index, or the first pixel of the column
        // if the index is beyond the top edge of the image
        let get_top = |i: usize| {
            if i < col_start {
                fv
            } else {
                backbuf[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the column
        // if the index is beyond the bottom edge of the image
        let get_bottom = |i: usize| {
            if i > col_end {
                lv
            } else {
                backbuf[i]
            }
        };

        for j in 0..min(blur_radius, height) {
            let bb = backbuf[ti + j * width];
            val_r += isize::from(bb[0]);
            val_g += isize::from(bb[1]);
            val_b += isize::from(bb[2]);
            val_a += isize::from(bb[3]);
        }
        if blur_radius > height {
            val_r += (blur_radius - height) as isize * isize::from(lv[0]);
            val_g += (blur_radius - height) as isize * isize::from(lv[1]);
            val_b += (blur_radius - height) as isize * isize::from(lv[2]);
            val_a += (blur_radius - height) as isize * isize::from(lv[3]);
        }

        for _ in 0..min(height, blur_radius + 1) {
            let bb = get_bottom(ri);
            ri += width;
            val_r += isize::from(bb[0]) - isize::from(fv[0]);
            val_g += isize::from(bb[1]) - isize::from(fv[1]);
            val_b += isize::from(bb[2]) - isize::from(fv[2]);
            val_a += isize::from(bb[3]) - isize::from(fv[3]);

            frontbuf[ti] = [
                (val_r as f32 * iarr) as u8,
                (val_g as f32 * iarr) as u8,
                (val_b as f32 * iarr) as u8,
                (val_a as f32 * iarr) as u8,
            ];
            ti += width;
        }

        if height > blur_radius {
            // otherwise `(height - blur_radius)` will underflow
            for _ in (blur_radius + 1)..(height - blur_radius) {
                let bb1 = backbuf[ri];
                ri += width;
                let bb2 = backbuf[li];
                li += width;

                val_r += isize::from(bb1[0]) - isize::from(bb2[0]);
                val_g += isize::from(bb1[1]) - isize::from(bb2[1]);
                val_b += isize::from(bb1[2]) - isize::from(bb2[2]);
                val_a += isize::from(bb1[3]) - isize::from(bb2[3]);

                frontbuf[ti] = [
                    (val_r as f32 * iarr) as u8,
                    (val_g as f32 * iarr) as u8,
                    (val_b as f32 * iarr) as u8,
                    (val_a as f32 * iarr) as u8,
                ];
                ti += width;
            }

            for _ in 0..min(height - blur_radius - 1, blur_radius) {
                let bb = get_top(li);
                li += width;

                val_r += isize::from(lv[0]) - isize::from(bb[0]);
                val_g += isize::from(lv[1]) - isize::from(bb[1]);
                val_b += isize::from(lv[2]) - isize::from(bb[2]);
                val_a += isize::from(lv[3]) - isize::from(bb[3]);

                frontbuf[ti] = [
                    (val_r as f32 * iarr) as u8,
                    (val_g as f32 * iarr) as u8,
                    (val_b as f32 * iarr) as u8,
                    (val_a as f32 * iarr) as u8,
                ];
                ti += width;
            }
        }
    }
}

#[inline]
fn box_blur_horz(
    backbuf: &[[u8; 4]],
    frontbuf: &mut [[u8; 4]],
    width: usize,
    height: usize,
    blur_radius: usize,
) {
    if blur_radius == 0 {
        frontbuf.copy_from_slice(backbuf);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..height {
        let row_start: usize = i * width; // inclusive
        let row_end: usize = (i + 1) * width - 1; // inclusive
        let mut ti: usize = i * width; // VERTICAL: $i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius;

        let fv: [u8; 4] = backbuf[row_start];
        let lv: [u8; 4] = backbuf[row_end]; // VERTICAL: $backbuf[ti + $width - 1];

        let mut val_r: isize = (blur_radius as isize + 1) * isize::from(fv[0]);
        let mut val_g: isize = (blur_radius as isize + 1) * isize::from(fv[1]);
        let mut val_b: isize = (blur_radius as isize + 1) * isize::from(fv[2]);
        let mut val_a: isize = (blur_radius as isize + 1) * isize::from(fv[3]);

        // Get the pixel at the specified index, or the first pixel of the row
        // if the index is beyond the left edge of the image
        let get_left = |i: usize| {
            if i < row_start {
                fv
            } else {
                backbuf[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the row
        // if the index is beyond the right edge of the image
        let get_right = |i: usize| {
            if i > row_end {
                lv
            } else {
                backbuf[i]
            }
        };

        for j in 0..min(blur_radius, width) {
            let bb = backbuf[ti + j]; // VERTICAL: ti + j * width
            val_r += isize::from(bb[0]);
            val_g += isize::from(bb[1]);
            val_b += isize::from(bb[2]);
            val_a += isize::from(bb[3]);
        }
        if blur_radius > width {
            val_r += (blur_radius - height) as isize * isize::from(lv[0]);
            val_g += (blur_radius - height) as isize * isize::from(lv[1]);
            val_b += (blur_radius - height) as isize * isize::from(lv[2]);
            val_a += (blur_radius - height) as isize * isize::from(lv[3]);
        }

        // Process the left side where we need pixels from beyond the left edge
        for _ in 0..min(width, blur_radius + 1) {
            let bb = get_right(ri);
            ri += 1;
            val_r += isize::from(bb[0]) - isize::from(fv[0]);
            val_g += isize::from(bb[1]) - isize::from(fv[1]);
            val_b += isize::from(bb[2]) - isize::from(fv[2]);
            val_a += isize::from(bb[3]) - isize::from(fv[3]);

            frontbuf[ti] = [
                (val_r as f32 * iarr) as u8,
                (val_g as f32 * iarr) as u8,
                (val_b as f32 * iarr) as u8,
                (val_a as f32 * iarr) as u8,
            ];
            ti += 1; // VERTICAL : ti += width, same with the other areas
        }

        if width > blur_radius {
            // otherwise `(width - blur_radius)` will underflow
            // Process the middle where we know we won't bump into borders
            // without the extra indirection of get_left/get_right. This is faster.
            for _ in (blur_radius + 1)..(width - blur_radius) {
                let bb1 = backbuf[ri];
                ri += 1;
                let bb2 = backbuf[li];
                li += 1;

                val_r += isize::from(bb1[0]) - isize::from(bb2[0]);
                val_g += isize::from(bb1[1]) - isize::from(bb2[1]);
                val_b += isize::from(bb1[2]) - isize::from(bb2[2]);
                val_a += isize::from(bb1[3]) - isize::from(bb2[3]);

                frontbuf[ti] = [
                    (val_r as f32 * iarr) as u8,
                    (val_g as f32 * iarr) as u8,
                    (val_b as f32 * iarr) as u8,
                    (val_a as f32 * iarr) as u8,
                ];
                ti += 1;
            }

            // Process the right side where we need pixels from beyond the right edge
            for _ in 0..min(width - blur_radius - 1, blur_radius) {
                let bb = get_left(li);
                li += 1;

                val_r += isize::from(lv[0]) - isize::from(bb[0]);
                val_g += isize::from(lv[1]) - isize::from(bb[1]);
                val_b += isize::from(lv[2]) - isize::from(bb[2]);
                val_a += isize::from(lv[3]) - isize::from(bb[3]);

                frontbuf[ti] = [
                    (val_r as f32 * iarr) as u8,
                    (val_g as f32 * iarr) as u8,
                    (val_b as f32 * iarr) as u8,
                    (val_a as f32 * iarr) as u8,
                ];
                ti += 1;
            }
        }
    }
}

#[inline]
fn box_blur_single_channel(
    backbuf: &mut [u8],
    frontbuf: &mut [u8],
    width: usize,
    height: usize,
    blur_radius_horz: usize,
    blur_radius_vert: usize,
) {
    box_blur_horz_single_channel(backbuf, frontbuf, width, height, blur_radius_horz);
    box_blur_vert_single_channel(frontbuf, backbuf, width, height, blur_radius_vert);
}

#[inline]
fn box_blur_vert_single_channel(
    backbuf: &[u8],
    frontbuf: &mut [u8],
    width: usize,
    height: usize,
    blur_radius: usize,
) {
    if blur_radius == 0 {
        frontbuf.copy_from_slice(backbuf);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..width {
        let col_start = i; //inclusive
        let col_end = i + width * (height - 1); //inclusive
        let mut ti: usize = i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius * width;

        let fv: u8 = backbuf[col_start];
        let lv: u8 = backbuf[col_end];

        let mut val_r: isize = (blur_radius as isize + 1) * isize::from(fv);

        // Get the pixel at the specified index, or the first pixel of the column
        // if the index is beyond the top edge of the image
        let get_top = |i: usize| {
            if i < col_start {
                fv
            } else {
                backbuf[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the column
        // if the index is beyond the bottom edge of the image
        let get_bottom = |i: usize| {
            if i > col_end {
                lv
            } else {
                backbuf[i]
            }
        };

        for j in 0..min(blur_radius, height) {
            let bb = backbuf[ti + j * width];
            val_r += isize::from(bb);
        }
        if blur_radius > height {
            val_r += (blur_radius - height) as isize * isize::from(lv);
        }

        for _ in 0..min(height, blur_radius + 1) {
            let bb = get_bottom(ri);
            ri += width;
            val_r += isize::from(bb) - isize::from(fv);

            frontbuf[ti] = (val_r as f32 * iarr) as u8;
            ti += width;
        }

        if height > blur_radius {
            // otherwise `(height - blur_radius)` will underflow
            for _ in (blur_radius + 1)..(height - blur_radius) {
                let bb1 = backbuf[ri];
                ri += width;
                let bb2 = backbuf[li];
                li += width;

                val_r += isize::from(bb1) - isize::from(bb2);

                frontbuf[ti] = (val_r as f32 * iarr) as u8;
                ti += width;
            }

            for _ in 0..min(height - blur_radius - 1, blur_radius) {
                let bb = get_top(li);
                li += width;

                val_r += isize::from(lv) - isize::from(bb);

                frontbuf[ti] = (val_r as f32 * iarr) as u8;
                ti += width;
            }
        }
    }
}

#[inline]
fn box_blur_horz_single_channel(
    backbuf: &[u8],
    frontbuf: &mut [u8],
    width: usize,
    height: usize,
    blur_radius: usize,
) {
    if blur_radius == 0 {
        frontbuf.copy_from_slice(backbuf);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..height {
        let row_start: usize = i * width; // inclusive
        let row_end: usize = (i + 1) * width - 1; // inclusive
        let mut ti: usize = i * width; // VERTICAL: $i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius;

        let fv: u8 = backbuf[row_start];
        let lv: u8 = backbuf[row_end]; // VERTICAL: $backbuf[ti + $width - 1];

        let mut val_r: isize = (blur_radius as isize + 1) * isize::from(fv);

        // Get the pixel at the specified index, or the first pixel of the row
        // if the index is beyond the left edge of the image
        let get_left = |i: usize| {
            if i < row_start {
                fv
            } else {
                backbuf[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the row
        // if the index is beyond the right edge of the image
        let get_right = |i: usize| {
            if i > row_end {
                lv
            } else {
                backbuf[i]
            }
        };

        for j in 0..min(blur_radius, width) {
            let bb = backbuf[ti + j]; // VERTICAL: ti + j * width
            val_r += isize::from(bb);
        }

        if blur_radius > width {
            val_r += (blur_radius - height) as isize * isize::from(lv);
        }

        // Process the left side where we need pixels from beyond the left edge
        for _ in 0..min(width, blur_radius + 1) {
            let bb = get_right(ri);
            ri += 1;
            val_r += isize::from(bb) - isize::from(fv);

            frontbuf[ti] = (val_r as f32 * iarr) as u8;
            ti += 1; // VERTICAL : ti += width, same with the other areas
        }

        if width > blur_radius {
            // otherwise `(width - blur_radius)` will underflow
            // Process the middle where we know we won't bump into borders
            // without the extra indirection of get_left/get_right. This is faster.
            for _ in (blur_radius + 1)..(width - blur_radius) {
                let bb1 = backbuf[ri];
                ri += 1;
                let bb2 = backbuf[li];
                li += 1;

                val_r += isize::from(bb1) - isize::from(bb2);

                frontbuf[ti] = (val_r as f32 * iarr) as u8;
                ti += 1;
            }

            // Process the right side where we need pixels from beyond the right edge
            for _ in 0..min(width - blur_radius - 1, blur_radius) {
                let bb = get_left(li);
                li += 1;

                val_r += isize::from(lv) - isize::from(bb);

                frontbuf[ti] = (val_r as f32 * iarr) as u8;
                ti += 1;
            }
        }
    }
}

// #[inline]
// /// Fast rounding for x <= 2^23.
// /// This is orders of magnitude faster than built-in rounding intrinsic.
// ///
// /// Source: https://stackoverflow.com/a/42386149/585725
// fn round(mut x: f32) -> f32 {
//     x += 12582912.0;
//     x -= 12582912.0;
//     x
// }
