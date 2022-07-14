use crate::streamer::{Streamers, VideoInfo, self};
use crate::upload::{BiliUpload, self};
use crate::{db, get_diff};
use async_recursion::async_recursion;
use async_trait::async_trait;
use biliup::video::Video;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tokio_stream::StreamExt;
use yaserde::de::from_str;
use yaserde_derive::YaDeserialize;

pub struct Youtube {
    pub client: ClientWithMiddleware,
    pub channel_id: String,
    pub videos: Vec<VideoInfo>,
    pub visitor_data: HashMap<String, String>,
    #[allow(dead_code)]
    pub info: BiliUpload,
}

#[derive(Debug, Clone)]
pub struct DownloadResponse {
    pub id: String,
    pub title: String,
    pub url: String,
    pub platform: String,
    pub author: String,
    pub published: String,
    pub description: String,
    pub channel_id: String,
    pub bilireq: BiliupReq,
}

#[derive(Debug, Clone)]
pub struct BiliupReq {
    pub title: String,
    pub filename: String,
    pub desc: String,
}

const CLIENT_NAME: &str = "WEB";
const CLIENT_VERSION: &str = "2.20220405";
// const CLIENT_NAME: &str = "ANDROID";
// const CLIENT_VERSION: &str = "17.13.3";
const USER_AGENT:&str="Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Mobile Safari/537.36Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Mobile Safari/537.36";
#[async_trait]
impl Streamers for Youtube {
    async fn get_new_videos(&mut self) -> Result<Vec<VideoInfo>, Box<dyn Error>> {
        let mut videos = Vec::new();
        let text = self
            .client
            .get(format!(
                "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                self.channel_id
            ))
            .send()
            .await?
            .text()
            .await?;
        // println!("{}", text);
        let loaded: Feed = from_str(&text)?;
        let author = loaded.author.name.clone();
        // println!("{:#?}", loaded.entry[0].group.content.url);
        let mut size: usize = 3;
        if loaded.entry.len() < 3 {
            size = loaded.entry.len();
        }
        let new_videos: Vec<Entry> = loaded.entry[0..size].to_vec();
        let old = db::new().get_new_videos(loaded.author.name, size as i32);
        let mut new = vec![];
        for v in new_videos.clone() {
            new.push(v.video_id.clone());
        }

        let diff = get_diff(old, new);

        for entry in new_videos {
            for mv in diff.clone() {
                if mv == entry.video_id {
                    videos.push(VideoInfo {
                        id: entry.video_id.clone(),
                        title: entry.title.clone(),
                        url: entry.group.content.url.clone(),
                        platform: "youtube".to_string(),
                        author: entry.author.name.clone(),
                        published: entry.published.clone(),
                        description: entry.group.description.clone(),
                        channel_id: self.channel_id.clone(),
                    })
                }
            }
        }
        self.videos = videos.clone();
        if videos.len() == 0 {
            println!("Paltform: youtube,{}: No new videos", author.clone());
        }
        return Ok(videos);
    }

    async fn get_real_video_url(&mut self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        let mut urls = Vec::new();
        let mut new_videos = vec![];
        let mut vidmap = std::collections::HashMap::new();
        for i in self.videos.iter() {
            new_videos.push(i.id.clone());
        }
        while new_videos.len() != 1 {
            new_videos.pop();
        }

        let mut stream = tokio_stream::iter(new_videos.into_iter());
        while let Some(v) = stream.next().await {
            let j = json!(
                {
                    "context": {
                        "client": {
                            "hl": "en",
                            "gl": "US",
                            "clientName": CLIENT_NAME,
                            "clientVersion": CLIENT_VERSION,
                            "clientScreen": "WATCH"
                        },
                        "thirdParty": {
                            "embedUrl": "https://www.youtube.com/"
                        }
                    },
                    "videoId": v,
                    "playbackContext": {
                        "contentPlaybackContext": {
                            "autonavState": "STATE_ON",
                            "html5Preference": "HTML5_PREF_WANTS",
                            "signatureTimestamp": 19075,
                            "lactMilliseconds": "-1"
                        }
                    },
                    "racyCheckOk": true,
                    "contentCheckOk": true
                }
            );

            let res: serde_json::Value = self.client
            .post("https://www.youtube.com/youtubei/v1/player?key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8")
            .header("Origin", "https://www.youtube.com")
            .header("Referer", "https://www.youtube.com/")
            .header("Accept-Language", "de,de-DE;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6")
            .header("User-Agent", USER_AGENT)
            .header(
                "X-Youtube-Identity-Token",
                "QUFFLUhqbWg4QWY3OXBVOEE2Wml0VmRpVFdub21jM2psQXw=",
            )
            .header("X-YouTube-Client-Name", CLIENT_NAME)
            .header("X-YouTube-Client-Version", CLIENT_VERSION)
            .json(&j)
            .send()
            .await
            .unwrap()
            .json()
            .await.unwrap();
            if res["streamingData"]["formats"].is_array() {
                let url = res["streamingData"]["formats"][0]["url"].as_str().unwrap();
                // println!("{}", url);
                urls.push(url.to_string());
                vidmap.insert(v.clone(), url.to_string());
                // return Ok(url.to_string());
                self.visitor_data.insert(
                    v.clone(),
                    res["responseContext"]["visitorData"]
                        .to_string()
                        .trim_matches('"')
                        .to_string()
                        .replace("%3D", "="),
                );
            }
        }
        Ok(vidmap)
    }

    async fn download(&mut self) -> Result<(), Box<dyn Error>> {
        let urls = self.get_real_video_url().await?;
        for url in urls.iter() {
            // self.client.get(url).send().await?.chunk();
            // 使用reqwest下载文件并保存为test.mp4
            println!("{:?},{:?}", url.0, url.1);
            // panic!("{:?}",self.visitor_data[url.0].clone());
            let resp = self
                .client
                .get(url.1)
                .header("User-Agent", USER_AGENT)
                .header("X-YouTube-Client-Name", CLIENT_NAME)
                .header("X-YouTube-Client-Version", CLIENT_VERSION)
                .header("X-Goog-Visitor-Id", self.visitor_data[url.0].clone())
                // .header(
                //     "X-Youtube-Identity-Token",
                //     "x=",
                // )
                // .header("X-Goog-PageId", "")
                .header("X-Origin", "https://www.youtube.com")
                .send()
                .await?;
            // println!("{},{}", resp.status(),resp.content_length().unwrap());
            match resp.status().is_success() {
                true => {
                    println!("{}", resp.status());
                    let filename = format!("{}.mp4", url.0);
                    let mut file = std::fs::File::create(Path::new(filename.as_str())).unwrap();
                    let total_size = resp.content_length().unwrap();
                    let mut downloaded: u64 = 0;
                    let mut stream = resp.bytes_stream();
                    let pb = ProgressBar::new(total_size);
                    pb.set_style(ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                    .progress_chars("#>-"));
                    pb.set_message("Downloading");
                    while let Some(item) = stream.next().await {
                        let chunk = item.or(Err(format!("Error while downloading file")))?;
                        file.write_all(&chunk)
                            .or(Err(format!("Error while writing to file")))?;
                        let new = min(downloaded + (chunk.len() as u64), total_size);
                        downloaded = new;
                        pb.set_position(new);
                    }
                    pb.finish_with_message("Downloaded Complete");
                }
                false => {
                    println!("{}", resp.status());
                }
            };
            // let content = resp.bytes().await?;
            // println!("{}", content);
            // print!("{}", resp.status());
            // let mut file = std::fs::File::create(Path::new("./test2.mp4"))?;
            // let mut content = Cursor::new(resp);
            // let ret = copy(&mut content, &mut file)?;
            // print!("{}", ret);
            // let mut pos = 0;
            // while pos < content.len() {
            //     let bytes_written = file.write(&content[pos..])?;
            //     pos += bytes_written;
            //     print!(".");
            // }
        }

        Ok(())
    }
}

impl Youtube {
    pub async fn _rustube_download(&mut self) -> Result<(), Box<dyn Error>> {
        let urls = self.get_real_video_url().await?;
        for url in urls.iter() {
            let yt_url = format!("https://www.youtube.com/watch?v={}", url.0);
            println!(
                "downloaded video to {:?}",
                rustube::download_best_quality(&yt_url).await.unwrap()
            );
        }
        Ok(())
    }
    #[allow(dead_code)]
    #[async_recursion]
    pub async fn ytdlp_download(&mut self) -> Result<String, Box<dyn Error>> {
        let urls = self.get_real_video_url().await?;
        for url in urls.iter() {
            let mut command = Command::new("yt-dlp");
            command.arg("-o");
            let video_dir = "./videos/";
            let file_dir_filename = format!("{}{}.mp4", video_dir, url.0);
            command.arg(&file_dir_filename);
            command.arg("-f");
            command.arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]");
            command.arg(url.0);
            command.arg("-R");
            command.arg("infinite");
            command.arg("--fragment-retries");
            command.arg("infinite");
            match command.status()?.code() {
                Some(code) => {
                    if code == 0 {
                        let res = command.output()?;
                        self.info.videos.push(Video {
                            title: Some(url.0.to_string()),
                            filename: file_dir_filename.to_string(),
                            desc: "".to_string(),
                        });
                        let _res = String::from_utf8(res.stdout)?;
                    } else {
                    }
                }
                None => {
                    println!("yt-dlp not found");

                    std::thread::sleep(std::time::Duration::from_secs(30));

                    self.ytdlp_download().await?;
                }
            }
        }
        Ok(String::new())
    }
}

pub async fn get_name_by_channel_id(
    client: ClientWithMiddleware,
    channel_id: String,
) -> Result<String, Box<dyn Error>> {
    let text = client
        .get(format!(
            "https://www.youtube.com/feeds/videos.xml?channel_id={}",
            channel_id
        ))
        .send()
        .await?
        .text()
        .await?;
    // println!("{}", text);
    let loaded: Feed = from_str(&text)?;
    // println!("{:#?}", loaded.entry[0].group.content.url);
    Ok(loaded.author.name.to_string())
}

pub async fn ytdlp_download(vf: VideoInfo) -> DownloadResponse {
    let bilireq = BiliupReq {
        title: vf.id.to_string().clone(),
        filename: "".to_string(),
        desc: "".to_string(),
    };
    let mut result = DownloadResponse {
        id: vf.id.clone(),
        title: vf.title.to_string(),
        url: vf.url.to_string(),
        platform: vf.platform.to_string(),
        author: vf.author.to_string(),
        published: vf.published.to_string(),
        description: vf.description.to_string(),
        channel_id: vf.channel_id.to_string(),
        bilireq: bilireq,
    };

    let mut command = Command::new("yt-dlp");
    command.arg("-o");
    let video_dir = "./videos/";
    let file_dir_filename = format!("{}{}.mp4", video_dir, vf.id);
    command.arg(&file_dir_filename);
    command.arg("-f");
    command.arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]");
    command.arg(vf.id.to_string());
    command.arg("-R");
    command.arg("infinite");
    command.arg("--fragment-retries");
    command.arg("infinite");
    match command.status().unwrap().code() {
        Some(code) => {
            if code == 0 {
                let _res = command.output().unwrap();
                // result
                // result.bilireq.desc ="".to_string();
                result.bilireq.filename = file_dir_filename.to_string();
                // result.bilireq.title = Some(vf.id.to_string());
                // result
                // let _res = String::from_utf8(res.stdout)?;
            } else {
                std::thread::sleep(std::time::Duration::from_secs(30));

                // ytdlp_download(vf);
            }
        }
        None => {
            println!("yt-dlp not found");

            std::thread::sleep(std::time::Duration::from_secs(30));

            // ytdlp_download(vf);
        }
    }
    result
}

pub fn new(channel_id: String) -> Youtube {
    return Youtube {
        client: streamer::new_client(),
        channel_id: channel_id,
        videos: Vec::new(),
        visitor_data: HashMap::new(),
        info: upload::new(),
    };
}

#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    rename = "feed",
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Feed {
    pub entry: Vec<Entry>,
    pub author: Author,
    pub title: String,
    pub id: String,
    pub published: String,
}
#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Author {
    pub name: String,
    pub uri: String,
}

#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Entry {
    pub id: String,
    #[yaserde(rename = "videoId", prefix = "yt")]
    pub video_id: String,
    pub title: String,
    pub link: Link,
    pub published: String,
    pub updated: String,
    #[yaserde(prefix = "media")]
    pub group: Group,
    pub author: Author,
}
#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Link {
    #[yaserde(attribute)]
    pub rel: String,
    #[yaserde(attribute)]
    pub href: String,
}
#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Group {
    #[yaserde(prefix = "media")]
    pub title: String,
    #[yaserde(prefix = "media")]
    pub thumbnail: String,
    #[yaserde(prefix = "media")]
    pub description: String,
    #[yaserde(prefix = "media")]
    pub content: Content,
}

#[derive(YaDeserialize, Debug, Clone, Default)]
#[yaserde(
    namespace = "yt: http://www.youtube.com/xml/schemas/2015"
    namespace = "media: http://search.yahoo.com/mrss/"
    namespace = "http://www.w3.org/2005/Atom"
  )]
pub struct Content {
    #[yaserde(attribute)]
    pub url: String,
    #[yaserde(attribute, rename = "type")]
    pub _type: String,
    #[yaserde(attribute)]
    pub width: String,
    #[yaserde(attribute)]
    pub height: String,
}
