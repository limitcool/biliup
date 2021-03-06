use async_trait::async_trait;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use std::{collections::HashMap, error::Error, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub platform: String,
    pub author: String,
    pub published: String,
    pub description: String,
    pub channel_id: String,
    // pub thumbnail: String,
}

#[async_trait]
pub trait Streamers {
    async fn get_new_videos(&mut self) -> Result<Vec<VideoInfo>, Box<dyn Error>>;
    async fn get_real_video_url(&mut self) -> Result<HashMap<String, String>, Box<dyn Error>>;
    async fn download(&mut self) -> Result<(), Box<dyn Error>>;
}

pub fn new_client() -> ClientWithMiddleware {
    // 设置最大重试次数为4294967295次
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(4294967295);
    let raw_client = reqwest::Client::builder()
        .cookie_store(true)
        // 设置超时时间为30秒
        .timeout(Duration::new(4294967295, 0))
        .build()
        .unwrap();
    let client = ClientBuilder::new(raw_client.clone())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    return client;
}

pub fn _select_streamers(cfg: crate::config::Streamers) -> Box<dyn Streamers> {
    match cfg.platform.to_lowercase().as_str() {
        "youtube" => Box::new(crate::youtube::new(cfg.url)),
        "twitch" => Box::new(crate::twitch::new(cfg.url)),
        _ => Box::new(Nop::default()),
    }
}

#[derive(Debug, Clone, Default)]
pub struct Nop {}
#[async_trait]
impl Streamers for Nop {
    async fn get_new_videos(&mut self) -> Result<Vec<VideoInfo>, Box<dyn Error>> {
        Ok(vec![])
    }
    async fn get_real_video_url(&mut self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        Ok(HashMap::new())
    }
    async fn download(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
