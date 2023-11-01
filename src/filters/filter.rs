use std::{time::{Instant, Duration}, collections::VecDeque, ops::Add};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum FilterBackend {
    GPU, //14 9 32::32 20 55            /speed improovements: 493% 170% 475%::272% 140% 329%
    CPU, //69 16 152::87 28 181
}

static PERF: Lazy<Mutex<VecDeque<Duration>>>= Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));
static PERF_W_MAP: Lazy<Mutex<VecDeque<Duration>>>= Lazy::new(|| Mutex::new(VecDeque::with_capacity(1001)));

/// works with 4 byte colors (RGBA/ARGB)
pub fn apply_blur(
    surface: &mut gdk::cairo::ImageSurface,
    sigma: f32,
    n: usize,
    backend: FilterBackend,
) -> Result<gdk::cairo::ImageSurface> {
    //TODO optimize (maybe remove some iter.collect but i'm too lazy to do it)
    //TODO ultimate optimization: use wgpu
    if sigma <= 0.0 {
        return Ok(surface.clone());
    }

    let (width, height) = (surface.width(), surface.height());

    let mut blurred_surface = gdk::cairo::ImageSurface::create(surface.format(), width, height)
        .with_context(|| "failed to create new blur imagesurface")?;
    let mut blurred_surface_data = blurred_surface
        .data()
        .with_context(|| "failed to get raw data from tmp blur surface")?;

    if sigma < height as f32 && sigma < width as f32 {
        let start= Instant::now();
        match backend {
            FilterBackend::GPU => {
                let surface_data = surface
                    .data()
                    .with_context(|| "failed to get raw data from tmp surface")?;
                let mut surface_data = surface_data.iter()
                    .map(|val| *val)
                    .collect::<Vec<u8>>();
                let data= surface_data.as_mut_slice();

                let start2=Instant::now();

                super::gpu_filter::GPU_INSTANCE.blocking_lock().gaussian_blur(data, width.try_into().unwrap(), height.try_into().unwrap(), sigma);

                let dur2= start.elapsed();
                {
                    const samples: u128=1000;
                    let mut perf=PERF.blocking_lock();
                    perf.push_back(dur2);
                    if perf.len()>samples as usize {
                        perf.pop_front();
                        let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
                        vec.sort();
                        let mut acc= 0u128;
                        for el in vec.iter() {
                            acc+=el;
                        }
                        let avg = Duration::from_micros((acc/samples).try_into().unwrap());
                        let p9=Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
                        let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
                        println!("RAW:{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}",backend, avg, p9,p99);
                    }
                }
                for i in 0..surface_data.len() {
                    blurred_surface_data[i] = surface_data[i];
                }
            }
            FilterBackend::CPU => {
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
                let start2=Instant::now();
                super::cpu_filter::gaussian_blur(
                    &mut surface_data,
                    width as usize,
                    height as usize,
                    sigma,
                    n,
                );
                let dur2= start.elapsed();
                {
                    const samples: u128=1000;
                    let mut perf=PERF.blocking_lock();
                    perf.push_back(dur2);
                    if perf.len()>samples as usize {
                        perf.pop_front();
                        let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
                        vec.sort();
                        let mut acc= 0u128;
                        for el in vec.iter() {
                            acc+=el;
                        }
                        let avg = Duration::from_micros((acc/samples).try_into().unwrap());
                        let p9=Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
                        let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
                        println!("RAW:{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}",backend, avg, p9,p99);
                    }
                }

                for i in 0..surface_data.len() {
                    blurred_surface_data2[i][0] = surface_data[i][0];
                    blurred_surface_data2[i][1] = surface_data[i][1];
                    blurred_surface_data2[i][2] = surface_data[i][2];
                    blurred_surface_data2[i][3] = surface_data[i][3];
                }
            }
        }
        let dur= start.elapsed();
        {
            const samples: u128=1000;
            let mut perf=PERF_W_MAP.blocking_lock();
            perf.push_back(dur);
            if perf.len()>samples as usize {
                perf.pop_front();
                let mut vec: Vec<u128> = perf.iter().map(|dur| dur.as_micros()).collect();
                vec.sort();
                let mut acc= 0u128;
                for el in vec.iter() {
                    acc+=el;
                }
                let avg = Duration::from_micros((acc/samples).try_into().unwrap());
                let p9=Duration::from_micros((*vec.get(90).unwrap()).try_into().unwrap());
                let p99 = Duration::from_micros((*vec.get(990).unwrap()).try_into().unwrap());
                println!("{:?} avg: {:?}, 9th p: {:?}, 99th p: {:?}",backend, avg, p9,p99);
            }
        }
    }

    
    drop(blurred_surface_data);
    blurred_surface.mark_dirty();
    Ok(blurred_surface)
}
