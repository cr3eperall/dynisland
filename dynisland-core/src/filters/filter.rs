// use std::{
//     collections::VecDeque,
//     time::{Duration, Instant},
// };

// use once_cell::sync::Lazy;
// use tokio::sync::Mutex;
use anyhow::{Context, Result};

#[derive(Debug)]
pub enum FilterBackend {
    Gpu,
    Cpu,
}

// static PERF: Lazy<Mutex<VecDeque<Duration>>> =
//     Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));
// static PERF_2: Lazy<Mutex<VecDeque<Duration>>> =
//     Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));

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

pub fn kernel_size_for_sigma(sigma: f32) -> u32 {
    2 * (sigma * 3.0).ceil() as u32 + 1
}
