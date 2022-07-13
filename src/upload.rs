use biliup::{
    client::Client,
    line::Probe,
    video::{Studio, Subtitle, Video},
    VideoFile,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use std::time::Instant;
#[derive(Debug)]
pub struct BiliUpload {
    pub desc: String,
    pub dynamic: String,
    pub subtitle: Subtitle,
    pub tag: String,
    pub title: String,
    pub videos: Vec<Video>,
    pub copyright: u8,
    pub source: String,
    pub tid: u16,
    pub cover: String,
    pub dtime: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BiliUpRespone {
    pub code: i64,
    pub message: String,
    #[serde(default = "default_aid")]
    pub bvid: String,
    #[serde(default = "default_aid")]
    pub aid: String,
}

fn default_aid() -> String {
    "Err".to_string()
}

pub async fn bili_upload(b: &BiliUpload) -> Result<BiliUpRespone, Box<dyn Error>> {
    let client = &Client::new();
    // let client = crate::streamer::new_client();

    let cookies_file = std::fs::File::options()
        .read(true)
        .write(true)
        .open(Path::new("cookies.json"))?;
    let login_info = client.login_by_cookies(cookies_file).await?;

    let line = Probe::probe().await.unwrap_or_default();

    // for videoinfo in  b.videos {
    // println!("{:?}", video_path.canonicalize()?.to_str());
    println!("{line:?}");
    let video_file = VideoFile::new(std::path::Path::new(&b.videos[0].filename))?;
    let total_size = video_file.total_size;
    let file_name = video_file.file_name.clone();
    let uploader = line.to_uploader(video_file);

    let instant = Instant::now();
    let limit: usize = 3;
    let video = uploader.upload(client, limit, |vs| vs).await?;
    let t = instant.elapsed().as_millis();
    println!(
        "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
        t as f64 / 1000.,
        total_size as f64 / 1000. / t as f64
    );
    // }

    println!("{:#?}", video);
    let _res = match Studio::builder()
        .desc(b.desc.clone())
        .dtime(b.dtime)
        .copyright(b.copyright)
        .cover(b.cover.clone())
        .dynamic(b.dynamic.clone())
        .source(b.source.clone())
        .tag(b.tag.clone())
        .tid(b.tid.clone())
        .title(b.title.clone())
        .videos(vec![video])
        .build()
        .submit(&login_info)
        .await
    {
        Ok(v) => {
            println!("OK:{:#?}", v);
            let res = BiliUpRespone {
                code: v["code"].as_i64().unwrap(),
                message: v["message"].to_string(),
                bvid: v["data"]["bvid"].to_string(),
                aid: v["data"]["aid"].to_string(),
            };
            return Ok(res);
        }
        Err(e) => {
            println!("Error:{:#?}", e.to_string());
            let b: BiliUpRespone = serde_json::from_str(e.to_string().as_str()).unwrap();
            // let v = x.as_object().unwrap();
            println!("ERR:{:#?},{:#?}", b.code, b.message);
            // println!("ERR:{:#?}",v["code"]);
            return Ok(b);
        }
    };
    // println!("93: {:#?}", res);

    // panic!("{:?}", res);
}

pub fn new() -> BiliUpload {
    BiliUpload {
        desc: "".to_string(),
        dynamic: "".to_string(),
        subtitle: Subtitle::default(),
        tag: "biliup,initcool".to_string(),
        title: "".to_string(),
        videos: vec![],
        copyright: 2,
        source: "https://github.com/limitcool/biliup".to_string(),
        tid: 17,
        cover: "".to_string(),
        dtime: None,
    }
}
