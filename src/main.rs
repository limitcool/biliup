// #[macro_use]
// extern crate lazy_static;
use biliup::video::{Subtitle, Video};
use config::Config;

use futures::StreamExt;
use youtube::DownloadResponse;

use std::{collections::HashMap, error::Error, path::Path};
use streamer::{Streamers, VideoInfo};
use tokio::sync::mpsc::{self, Receiver, Sender};
use upload::BiliUpload;
mod config;
mod db;
mod streamer;
mod twitch;
mod upload;
mod youtube;

// lazy_static!{
//     static ref MPSC:(tokio::sync::mpsc::Sender<Arc<BiliUpload>>,tokio::sync::mpsc::Receiver<Arc<BiliUpload>>) = mpsc::channel(32);
// }

#[derive(Debug)]
pub struct BiliupVideos {
    pub biliupload: BiliUpload,
    pub author: String,
    pub title: String,
    pub url: String,
    pub platform: String,
    pub video_id: String,
}

#[tokio::main]
async fn main() {
    let mut db: db::DB = db::new();
    db.create_db().unwrap();
    let cfg = load_config(Path::new("config.yaml")).unwrap();
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async {
        upload(rx).await;
    });
    let (dtx, mut drx): (Sender<VideoInfo>, Receiver<VideoInfo>) = mpsc::channel(10);
    tokio::spawn(async move {
        while let Some(vi) = drx.recv().await {
            match vi.platform.clone().as_str() {
                "youtube" => {
                    let dr = youtube::ytdlp_download(vi).await;
                    tx.clone().send(generate_biliup_videos(dr)).await.unwrap();
                }
                "twitch" => {
                    let dr = twitch::ytdlp_download(vi).await;
                    tx.clone().send(generate_biliup_videos(dr)).await.unwrap();
                }
                _ => {}
            }
        }
    });
    loop {
        let authors = tokio_stream::iter(cfg.streamers.iter().clone());
        authors
            .for_each_concurrent(5, |v| async {
                match v.1.platform.to_lowercase().as_str() {
                    "youtube" => {
                        let videos = youtube_get_new_video(v.1.url.clone()).await;
                        for v in videos {
                            dtx.send(v).await.unwrap();
                        }
                    }
                    "twitch" => {
                        let mut r = twitch::new(v.1.url.clone());
                        let videos = r.get_new_videos().await.unwrap();

                        for v in videos.clone() {
                            db.insert_author(v.author.clone());
                            let db = db::new();
                            db.insert_video_id(
                                v.author.clone(),
                                v.id.clone(),
                                v.title.clone(),
                                v.url.clone(),
                                v.platform.clone(),
                                "unknown".to_string(),
                                "unknown".to_string(),
                                "unknown".to_string(),
                                "unknown".to_string(),
                            );
                            dtx.send(v).await.unwrap();
                        }
                    }
                    _ => {
                        print!("{}", v.0.clone());
                    }
                };
            })
            .await;
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

pub fn load_config(config: &Path) -> Result<Config, Box<dyn Error>> {
    let file = std::fs::File::open(config)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

pub fn get_platform(cfg: String) -> String {
    match cfg.to_lowercase().as_str() {
        "youtube" => "youtube".to_string(),
        "twitch" => "twitch".to_string(),
        _ => "unknown".to_string(),
    }
}

// 传入旧,新两个数组,判断两个数组中的字符串是否相同,返回新数组中不同的字符串
pub fn get_diff(old: Vec<String>, new: Vec<String>) -> Vec<String> {
    let mut diff = vec![];
    for i in new {
        if !old.contains(&i) {
            diff.push(i);
        }
    }
    diff
}

pub async fn upload(mut rx: Receiver<BiliupVideos>) {
    println!("upload");
    while let Some(bv) = rx.recv().await {
        println!("收到新内容:{:?}", bv.author);
        let resp = upload::bili_upload(&bv.biliupload).await.unwrap();
        let db: db::DB = db::new();
        db.update_submit_info(
            resp.bvid,
            resp.aid,
            resp.message,
            resp.code.to_string(),
            bv.video_id,
        );
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
    println!("upload end");
}

// 从youtube获取新视频
pub async fn youtube_get_new_video(channel_id: String) -> Vec<streamer::VideoInfo> {
    let mut vs = vec![];
    let db = db::new();
    let author_name = youtube::get_name_by_channel_id(streamer::new_client(), channel_id.clone())
        .await
        .unwrap();
    db.insert_author(author_name.clone());
    let mut r = youtube::new(channel_id.clone());
    let vf = r.get_new_videos().await.unwrap();
    for v in vf {
        let db: db::DB = db::new();
        db.insert_video_id(
            v.author.clone(),
            v.id.clone(),
            v.title.clone(),
            v.url.clone(),
            v.platform.clone(),
            "unknown".to_string(),
            "unknown".to_string(),
            "unknown".to_string(),
            "unknown".to_string(),
        );
        vs.push(v.clone());
    }
    return vs;
}

// 生成提交B站的视频信息
pub fn generate_biliup_videos(dr: DownloadResponse) -> BiliupVideos {
    let bu = BiliUpload {
        desc: dr.description.clone(),
        dynamic: "".to_string(),
        subtitle: Subtitle::default(),
        tag: "biliup,initcool".to_string(),
        title: dr.title.clone(),
        videos: vec![Video {
            title: Some(dr.bilireq.title.clone()),
            filename: dr.bilireq.filename.clone(),
            desc: "".to_string(),
        }],
        copyright: 2,
        source: "https://github.com/limitcool/biliup".to_string(),
        tid: 17,
        cover: "".to_string(),
        dtime: None,
    };
    let biliup_videos = BiliupVideos {
        biliupload: bu,
        author: dr.author.clone(),
        title: dr.title.clone(),
        url: dr.url.clone(),
        platform: dr.platform.clone(),
        video_id: dr.id.clone(),
    };
    return biliup_videos;
}
