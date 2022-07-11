use config::Config;
use regex::Regex;
use rusqlite::{Connection, Result};
use std::{collections::HashMap, error::Error, path::Path};
use streamer::Streamers;
use tokio_stream::StreamExt;
mod config;
mod streamer;
mod upload;
mod youtube;
#[tokio::main]
async fn main() {
    let conn = Connection::open("./biliup.db").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS streamers (
            id INTEGER PRIMARY KEY,
            author TEXT NOT NULL UNIQUE
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY,
            streamer_author TEXT NOT NULL,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            platform TEXT NOT NULL,
            video_id TEXT NOT NULL,
            bv_id TEXT NOT NULL,
            aid TEXT NOT NULL,
            biliup_message TEXT NOT NULL,
            biliup_code TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT (datetime('now','localtime')),
            updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now','localtime')),
            UNIQUE (title, url,video_id),
            FOREIGN KEY(streamer_author) REFERENCES streamers(author)
        )",
        [],
    )
    .unwrap();
    let cfg = load_config(Path::new("config.yaml")).unwrap();
    // let mut r = youtube::Youtube {
    //     client: streamer::new_client(),
    //     channel_id: "UCee3jrGUdb2ovrE7v4ncH3Q".to_string(),
    //     videos: Vec::new(),
    //     visitor_data: HashMap::new(),
    //     info: upload::new(),
    // };
    let mut streamers = tokio_stream::iter(cfg.streamers.iter()).take(3);
    while let Some(v) = streamers.next().await {
        // 如果记录不存在,就插入,已经存在不插入
        conn.execute("INSERT OR IGNORE INTO streamers (author) VALUES (?1)", [v.0]).unwrap();

        let mut r = youtube::Youtube {
            client: streamer::new_client(),
            channel_id: v.1.url.clone(),
            videos: Vec::new(),
            visitor_data: HashMap::new(),
            info: upload::new(),
        };
        r.get_new_videos().await.unwrap();
        r.ytdlp_download().await.unwrap();
        let resp = upload::bili_upload(r.info).await.unwrap();
        conn.execute(
            "INSERT INTO videos (streamer_author,title,url,platform,video_id,bv_id,aid,biliup_message,biliup_code) VALUES (?,?,?,?,?,?,?,?,?)",
            &[
                v.0,
                &r.videos[0].title,
                &r.videos[0].url,
                &r.videos[0].platform,
                &r.videos[0].id,
                &resp.bvid,
                &resp.aid,
                &resp.message,
                &resp.code.to_string(),
            ],
        ).unwrap();
    }
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
