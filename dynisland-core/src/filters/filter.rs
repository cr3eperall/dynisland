// use std::{
//     collections::VecDeque,
//     time::{Duration, Instant},
// };

// use once_cell::sync::Lazy;
// use tokio::sync::Mutex;
use anyhow::{Context, Ok, Result};

#[derive(Debug)]
pub enum FilterBackend {
    Gpu,
    Cpu,
}

// static PERF: Lazy<Mutex<VecDeque<Duration>>> =
//     Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));
// static PERF_2: Lazy<Mutex<VecDeque<Duration>>> =
//     Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));

/// outputs the result on surface1
pub fn apply_blur_and_merge_opacity_dual(
    surface_1: &mut gdk::cairo::ImageSurface,
    surface_2: &mut gdk::cairo::ImageSurface,
    sigma_1: f32,
    sigma_2: f32,
    opacity_1: f32,
    opacity_2: f32,
    backend: FilterBackend,
) -> Result<()> {
    let (width, height) = (surface_1.width(), surface_1.height());
    if width != surface_2.width() || height != surface_2.height() {
        panic!("images have different sizes")
    }

    let mut surface_data_1 = surface_1
        .data()
        .with_context(|| "failed to get raw data from tmp surface_1")?;
    let mut surface_data_2 = surface_2
        .data()
        .with_context(|| "failed to get raw data from tmp surface_2")?;

    if sigma_1 < height as f32
        && sigma_1 < width as f32
        && sigma_2 < height as f32
        && sigma_2 < width as f32
    {
        // let start = Instant::now();
        match backend {
            FilterBackend::Gpu => {
                // let start2=Instant::now();

                super::gpu_filter::WGPU_INSTANCE
                    .blocking_lock()
                    .apply_blur_and_merge_opacity_dual(
                        &mut surface_data_1,
                        &mut surface_data_2,
                        (width.try_into().unwrap(), height.try_into().unwrap()),
                        sigma_1,
                        sigma_2,
                        opacity_1,
                        opacity_2,
                    )
            }
            FilterBackend::Cpu => {
                // let n = kernel_size_for_sigma(sigma);

                // let mut surface_data = surface_data
                //     .chunks_exact(4)
                //     .map(|val| val.try_into().unwrap())
                //     .collect::<Vec<[u8; 4]>>();
                // // let start2=Instant::now();
                // super::cpu_filter::gaussian_blur(
                //     &mut surface_data,
                //     width as usize,
                //     height as usize,
                //     sigma,
                //     n.try_into().unwrap(),
                // );
                unimplemented!("maybe i will implement it one day");
            }
        }
    }
    drop(surface_data_1);
    drop(surface_data_2);
    surface_1.mark_dirty();
    Ok(())
}

/// works with 4 byte colors
pub fn apply_blur(
    surface: &mut gdk::cairo::ImageSurface,
    sigma: f32,
    backend: FilterBackend,
) -> Result<()> {
    if sigma <= 0.0 {
        return Ok(());
    }

    let (width, height) = (surface.width(), surface.height());

    let mut surface_data = surface
        .data()
        .with_context(|| "failed to get raw data from tmp surface")?;

    if sigma < height as f32 && sigma < width as f32 {
        // let start = Instant::now();
        match backend {
            FilterBackend::Gpu => {
                // let start2=Instant::now();

                super::gpu_filter::WGPU_INSTANCE
                    .blocking_lock()
                    .gaussian_blur(
                        &mut surface_data,
                        width.try_into().unwrap(),
                        height.try_into().unwrap(),
                        sigma,
                    );

                // let dur2= start2.elapsed();
                // {
                //     const samples: u128=1000;
                //     let mut perf=PERF.blocking_lock();
                //     perf.push_back(dur2);
                //     if perf.len()>samples as usize {
                //         perf.pop_front();
                //         let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
                //         vec.sort();
                //         let mut acc= 0u128;
                //         for el in vec.iter() {
                //             acc+=el;
                //         }
                //         let avg = Duration::from_micros((acc/samples).try_into().unwrap());
                //         let p9=Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
                //         let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
                //         let p999=Duration::from_micros((*vec.get(999).unwrap()).try_into().unwrap());
                //         println!("RAW:{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}, 999th p: {:?}",backend, avg, p9, p99, p999);
                //     }
                // }
            }
            FilterBackend::Cpu => {
                let n = kernel_size_for_sigma(sigma);

                let mut surface_data = surface_data
                    .chunks_exact(4)
                    .map(|val| val.try_into().unwrap())
                    .collect::<Vec<[u8; 4]>>();
                // let start2=Instant::now();
                super::cpu_filter::gaussian_blur(
                    &mut surface_data,
                    width as usize,
                    height as usize,
                    sigma,
                    n.try_into().unwrap(),
                );
                // let dur2= start2.elapsed();
                // {
                //     const samples: u128=1000;
                //     let mut perf=PERF.blocking_lock();
                //     perf.push_back(dur2);
                //     if perf.len()>samples as usize {
                //         perf.pop_front();
                //         let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
                //         vec.sort();
                //         let mut acc= 0u128;
                //         for el in vec.iter() {
                //             acc+=el;
                //         }
                //         let avg = Duration::from_micros((acc/samples).try_into().unwrap());
                //         let p9=Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
                //         let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
                //         let p999=Duration::from_micros((*vec.get(999).unwrap()).try_into().unwrap());
                //         println!("RAW:{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}, 999th p: {:?}",backend, avg, p9, p99, p999);
                //     }
                // }
            }
        }
        // let dur = start.elapsed();
        // {
        //     const SAMPLES: u128 = 1000;
        //     let mut perf = PERF_2.blocking_lock();
        //     perf.push_back(dur);
        //     if perf.len() > SAMPLES as usize {
        //         perf.pop_front();
        //         let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
        //         vec.sort();
        //         let mut acc = 0u128;
        //         for el in vec.iter() {
        //             acc += el;
        //         }
        //         let avg = Duration::from_micros((acc / SAMPLES).try_into().unwrap());
        //         let p9 = Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
        //         let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
        //         let p999 = Duration::from_micros((*vec.get(999).unwrap()).try_into().unwrap());
        //         println!(
        //             "{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}, 999th p: {:?}",
        //             backend, avg, p9, p99, p999
        //         );
        //     }
    }

    // drop(blurred_surface_data);
    // blurred_surface.mark_dirty();
    // Ok(blurred_surface)
    drop(surface_data);
    surface.mark_dirty();
    Ok(())
}

const SIGMA_MULT: f32=3.0;

pub fn kernel_size_for_sigma(sigma: f32) -> u32 {
    2 * (sigma * SIGMA_MULT).ceil() as u32 + 1
}
