use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use std::{collections::HashMap, error::Error, process::Command};

use crate::{streamer::{self, Streamers, VideoInfo}, youtube::{DownloadResponse, BiliupReq}, get_diff};

#[derive(Debug, Clone)]
pub struct Twitch {
    pub client: ClientWithMiddleware,
    pub id: String,
}

#[async_trait]
impl Streamers for Twitch {
    async fn get_new_videos(&mut self) -> Result<Vec<VideoInfo>, Box<dyn Error>> {
        let j = json!(
            [
                {
                  "operationName": "RoleRestricted",
                  "variables": {
                    "contentOwnerLogin": self.id
                  },
                  "extensions": {
                    "persistedQuery": {
                      "version": 1,
                      "sha256Hash": "7f57264e30ae6d9daa154bb62c8b0bcb1b38fc0b53a7b3cdecd60a060ff8332b"
                    }
                  }
                },
                {
                  "operationName": "ChannelVideoShelvesQuery",
                  "variables": {
                    "channelLogin": self.id,
                    "first": 5
                  },
                  "extensions": {
                    "persistedQuery": {
                      "version": 1,
                      "sha256Hash": "8afefb1ed16c4d8e20fa55024a7ed1727f63b6eca47d8d33a28500770bad8479"
                    }
                  }
                }
              ]
        );
        let res: serde_json::Value = self
            .client
            .post("https://gql.twitch.tv/gql")
            .header("client-id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
            .json(&j)
            .send()
            .await?
            .json()
            .await?;
        // println!("{:?}",res[1]["data"]["user"]["videoShelves"]["edges"]["node"]["items"]);
        // edges后的0为 近期直播 1 为 近期精选内容和上传视频 2为 热门剪辑 items后的0为第一个视频,以此类推
        // println!("{:?}",res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]);
        // panic!("");
        let v = VideoInfo {
            id: res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]["id"]
                .to_string()
                .replace("\"", ""),
            title: res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]["title"]
                .to_string()
                .replace("\"", ""),
            url: format!(
                "https://www.twitch.tv/videos/{}",
                res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]["id"]
                    .to_string()
                    .replace("\"", "")
            ),
            platform: "twitch".to_string(),
            author: res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]["owner"]
                ["displayName"]
                .to_string()
                .replace("\"", ""),
            published: res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]
                ["publishedAt"]
                .to_string()
                .replace("\"", ""),
            description: "".to_string(),
            channel_id: res[1]["data"]["user"]["videoShelves"]["edges"][0]["node"]["items"][0]
                ["id"]
                .to_string()
                .replace("\"", ""),
        };
        let db = crate::db::new();
        let old = db.get_new_videos(v.author.clone(), 1);
        if get_diff(old, vec![v.id.clone()]).len() == 0 {
            println!("Platform: [{}],{}: No new videos",v.platform, v.author.clone());
            return Ok(vec![]);
        }
        Ok(vec![v])
    }
    async fn get_real_video_url(&mut self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        Ok(HashMap::new())
    }
    async fn download(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub fn new(channel_id: String) -> Twitch {
    Twitch {
        client: streamer::new_client(),
        id: channel_id,
    }
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
    let file_dir_filename = format!("{}twitch_{}.mp4", video_dir, vf.id);
    command.arg(&file_dir_filename);
    command.arg("-f");
    command.arg("best[height>=720]");
    command.arg(vf.url.to_string());
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
                println!("我要等待30秒");
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                // ytdlp_download(vf);
            }
        }
        None => {
            println!("yt-dlp not found");

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            // ytdlp_download(vf);
        }
    }
    result
}