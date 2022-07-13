// #[macro_use]
// extern crate lazy_static;
use biliup::video::{Subtitle, Video};
use config::Config;
use futures::StreamExt;
use regex::Regex;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, error::Error, path::Path};
use streamer::Streamers;
use tokio::sync::mpsc::{self, Receiver};
use upload::BiliUpload;
mod config;
mod db;
mod streamer;
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
    let videos: Arc<Mutex<Vec<streamer::VideoInfo>>> = Arc::new(Mutex::new(vec![]));
    loop {
        let authors = tokio_stream::iter(cfg.streamers.iter().clone());
        authors
            .for_each_concurrent(5, |v| async {
                let db: db::DB = db::new();
                let author_name = youtube::get_name_by_channel_id(
                    streamer::new_client(),
                    v.1.url.clone().to_string(),
                )
                .await
                .unwrap();
                db.insert_author(author_name.clone());

                let mut r = youtube::Youtube {
                    client: streamer::new_client(),
                    channel_id: v.1.url.clone(),
                    videos: Vec::new(),
                    visitor_data: HashMap::new(),
                    info: upload::new(),
                };
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
                    videos.lock().unwrap().push(v);
                }
            })
            .await;
        if videos.lock().unwrap().len() > 0 {
            let vi = videos
                .lock()
                .unwrap()
                .clone()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let vs = tokio_stream::iter(vi);
            vs.for_each_concurrent(5, |v| async {
                let _r = youtube::Youtube {
                    client: streamer::new_client(),
                    channel_id: v.channel_id.clone(),
                    videos: Vec::new(),
                    visitor_data: HashMap::new(),
                    info: upload::new(),
                };
                let dr = youtube::ytdlp_download(v).await;
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
                println!("我出来了,发送任务",);
                tx.clone().send(biliup_videos).await.unwrap();
            })
            .await;
            videos.lock().unwrap().clear();
            
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
    // panic!();

    // tokio::spawn(upload(rx));

    // panic!();
    // let v = r.get_new_videos().await.unwrap();

    // r.ytdlp_download().await.unwrap();

    // let mut stream = tokio_stream::iter(new_videos);
    // while let Some(v) = stream.next().await {
    //     let v = v.unwrap();

    //     }
    // println!("{:?}", cfg);
    // let mut authors= vec![];
    // let a  = Arc::new(authors.clone());
    // cfg.streamers.into_iter().for_each(|(k, _)| {
    // println!("{}", k);
    // // 如果记录不存在,就插入,已经存在不插入
    // conn.execute("INSERT OR IGNORE INTO streamers (author) VALUES (?1)", [k])
    // .unwrap();
    // // 如果记录不存在,就插入,存在就更新
    // conn.execute("INSERT OR REPLACE INTO streamers (author) VALUES (?1)", [k]).unwrap();
    // conn.execute("INSERT INTO streamers(author) VALUES (?1)", [k]).unwrap();
    // authors.push(k);
    // tokio::spawn(async move {
    //     println!("{}", k);

    // });
    // });
    // into iter直接拿owned
    // for i in  authors.into_iter() {
    //     tokio::spawn(async move {
    //         println!("{}", i);

    //     });
    // }

    // let youtube_re = Regex::new(r"^https://www.youtube.com/.*").unwrap();
    // let youtube_re1 = Regex::new(r"^https://www.youtube.com/watch?v=(.*)").unwrap();
    // let youtube_re2 = Regex::new(r"^https://www.youtube.com/channel/(.*)").unwrap();
    // let youtube_re3 = Regex::new(r"^https://www.youtube.com/user/(.*)").unwrap();
    // let youtube_re4 = Regex::new(r"^https://www.youtube.com/playlist/(.*)").unwrap();
    // let twitch_re = Regex::new(r"^https://www.twitch.tv/.*").unwrap();
    // let twitch_re1 = Regex::new(r"^https://www.twitch.tv/videos/(.*)").unwrap();
    // let twitch_re2 = Regex::new(r"^https://www.twitch.tv/videos/v_(.*)").unwrap();
    // let twitch_re3 = Regex::new(r"^https://www.twitch.tv/videos/clip/(.*)").unwrap();
    // let mut stream = tokio_stream::iter(cfg.streamers.into_iter());
    // while let Some(v) = stream.next().await {
    //     match v.1.url.as_str() {
    //         x if youtube_re.is_match(x)
    //             | youtube_re1.is_match(x)
    //             | youtube_re2.is_match(x)
    //             | youtube_re3.is_match(x)
    //             | youtube_re4.is_match(x) =>
    //         {
    //             println!("{}", x);
    //         }
    //         x if twitch_re.is_match(x)
    //             | twitch_re1.is_match(x)
    //             | twitch_re2.is_match(x)
    //             | twitch_re3.is_match(x) =>
    //         {
    //             println!("{}", x);
    //         }
    //         _ => {
    //             panic!("配置文件错误");
    //         }
    //     };
    //     // println!("{}", platform);
    // }
}

pub fn load_config(config: &Path) -> Result<Config, Box<dyn Error>> {
    let file = std::fs::File::open(config)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

pub fn get_platform(url: &str) -> String {
    let youtube_re = Regex::new(r"https://www.youtube.com/.*").unwrap();
    let youtube_re1 = Regex::new(r"https://www.youtube.com/watch?v=(.*)").unwrap();
    let youtube_re2 = Regex::new(r"https://www.youtube.com/channel/(.*)").unwrap();
    let youtube_re3 = Regex::new(r"https://www.youtube.com/user/(.*)").unwrap();
    let youtube_re4 = Regex::new(r"https://www.youtube.com/playlist/(.*)").unwrap();
    let twitch_re = Regex::new(r"https://www.twitch.tv/.*").unwrap();
    let twitch_re1 = Regex::new(r"https://www.twitch.tv/videos/(.*)").unwrap();
    let twitch_re2 = Regex::new(r"https://www.twitch.tv/videos/v_(.*)").unwrap();
    let twitch_re3 = Regex::new(r"https://www.twitch.tv/videos/clip/(.*)").unwrap();

    if youtube_re.is_match(url)
        || youtube_re1.is_match(url)
        || youtube_re2.is_match(url)
        || youtube_re3.is_match(url)
        || youtube_re4.is_match(url)
    {
        return "youtube".to_string();
    } else if twitch_re.is_match(url)
        || twitch_re1.is_match(url)
        || twitch_re2.is_match(url)
        || twitch_re3.is_match(url)
    {
        return "twitch".to_string();
    } else {
        return "unknown".to_string();
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

// pub fn ffmpeg(rtmp_url:String,rtmp_key:String,m3u8_url:String){
//     let mut command =Command::new("ffmpeg");
//     command.arg("-re");
//     command.arg("-i");
//     command.arg(m3u8_url);
//     command.arg("-vcodec");
//     command.arg("copy");
//     command.arg("-acodec");
//     command.arg("aac");
//     command.arg("-f");
//     command.arg("flv");
//     command.arg(cmd);
//     match command.status().unwrap().code() {
//     Some(code) => {
//         println!("Exit Status: {}", code);
//     }
//     None => {
//         println!("Process terminated.");
//     }
// }
// }

pub async fn upload(mut rx: Receiver<BiliupVideos>) {
    println!("upload");
    while let Some(bv) = rx.recv().await {
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

pub async fn test() {
    print!("test");
}
