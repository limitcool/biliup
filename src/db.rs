// use std::error::Error;

use rusqlite::{Connection, Result};

#[derive(Debug)]
pub struct DB {
    pub conn: Connection,
}
impl DB {
    #[allow(dead_code)]
    pub fn create_db(&mut self) -> Result<()> {
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
    pub fn insert_submit_info(
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

    pub fn insert_author(&self, author: String) {
        // 如果记录不存在,就插入,已经存在不插入
        self.conn
            .execute(
                "INSERT OR IGNORE INTO streamers (author) VALUES (?1)",
                [author],
            )
            .unwrap();
    }
}

pub fn new() -> DB {
    let conn = Connection::open("./biliup.db").unwrap();
    DB { conn: conn }
}
