use std::{fs, io::Read};

pub async fn get_album_art_from_url(url:&str)-> Option<Vec<u8>>{
    // log::warn!("getting album art");
    let vec: Vec<u8>;
    if url.starts_with("http"){ //TODO better check for http or https
        vec=reqwest::Client::new().get(url).send().await.unwrap().bytes().await.unwrap().to_vec();
    }else if let Some(path)=url.strip_prefix("file://") {
        let mut buf=vec![0_u8; 26214400];
        let size = fs::File::open(path).ok()?.read(&mut buf).ok()?;
        vec=buf[..size].to_vec();
    } else {
        return None;
    }
    Some(vec)
}