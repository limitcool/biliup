use std::error::Error;

use rusqlite::{Connection, Result};
#[derive(Debug)]
pub struct DB {
    pub conn: Connection,
}
impl DB {
    #[allow(dead_code)]
    pub fn create_db(&mut self) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS streamers (
                    id INTEGER PRIMARY KEY,
                    author TEXT NOT NULL UNIQUE
                )",
            [],
        )?;
        self.conn.execute(
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
                    UNIQUE (url,video_id),
                    FOREIGN KEY(streamer_author) REFERENCES streamers(author)
                )",
            [],
        )?;
        Ok(())
    }
    pub fn _insert_submit_info(
        self,
        streamer_author: String,
        title: String,
        url: String,
        platform: String,
        video_id: String,
        bv_id: String,
        aid: String,
        biliup_message: String,
        biliup_code: String,
    ) {
        self.conn.execute(
            "INSERT INTO videos (streamer_author,title,url,platform,video_id,bv_id,aid,biliup_message,biliup_code) VALUES (?,?,?,?,?,?,?,?,?)",
            [
                streamer_author,
                title,
                url,
                platform,
                video_id,
                bv_id,
                aid,
                biliup_message,
                biliup_code,
            ],
        ).unwrap();
    }
    pub fn update_submit_info(
        self,
        bv_id: String,
        aid: String,
        biliup_message: String,
        biliup_code: String,
        video_id: String,
    ) {
        self.conn.execute(
            "UPDATE videos SET bv_id=?,aid=?,biliup_message=?,biliup_code=?,updated_at=(datetime('now','localtime')) WHERE video_id = ?",
            [
                bv_id.replace("\"", ""),
                aid,
                biliup_message,
                biliup_code,
                video_id
            ],
        ).unwrap();
    }
    pub fn insert_video_id(
        self,
        author: String,
        video_id: String,
        title: String,
        url: String,
        platform: String,
        bv_id: String,
        aid: String,
        biliup_message: String,
        biliup_code: String,
    ) {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO videos (streamer_author,video_id,title,url,platform,bv_id,aid,biliup_message,biliup_code) VALUES (?,?,?,?,?,?,?,?,?)",
                [
                    author,
                    video_id,
                    title,
                    url,
                    platform,
                    bv_id,
                    aid,
                    biliup_message,
                    biliup_code,
                ],
            )
            .unwrap();
    }
    pub fn insert_author(&self, author: String) {
        // 如果记录不存在,就插入,已经存在不插入
        self.conn
            .execute(
                "INSERT OR IGNORE INTO streamers (author) VALUES (?1)",
                [author],
            )
            .unwrap();
    }

    pub fn get_new_videos(&self, author: String,size:i32) -> Vec<String> {
        // self.conn
        //     .execute(
        //         "SELECT video_id FROM videos WHERE streamer_author = ? AND biliup_code = '0'",
        //         [author],
        //     )
        //     .unwrap();
        // 查询数据库,搜索指定作者的所有video_id,并返回
        let mut stmt = self.conn.prepare("SELECT video_id FROM videos WHERE streamer_author = ? AND biliup_code != 'unknown'  ORDER BY created_at DESC LIMIT ?").unwrap();
        let mut rows = stmt.query(&[&author,&size.to_string()]).unwrap();
        let mut video_ids = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let video_id = row.get(0).unwrap();
            video_ids.push(video_id);
        }
        video_ids
    }
}

pub fn new() -> DB {
    let conn = Connection::open("./biliup.db").unwrap();
    DB { conn: conn }
}
