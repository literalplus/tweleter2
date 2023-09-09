use crate::ImportParams;
use anyhow::*;
use rusqlite::Connection;
use serde_json::Value;

pub fn run(params: ImportParams) -> Result<()> {
    let conn = Connection::open("tweets.db")?;

    conn.execute(
        "create table if not exists tweets (id varchar(255) primary key, is_rt varchar(255) not null)",
        [],
    ).expect("create table");

    let infile = std::fs::File::open(params.file).expect("during open infile");
    let tweets: Vec<Value> = serde_json::from_reader(infile)?;

    for tweet_wrapper in tweets {
        let tweet = tweet_wrapper.get("tweet").expect("no tweet obj");
        let id = tweet
            .get("id")
            .expect("no tweet id")
            .as_str()
            .expect("id str");
        let full_text = tweet
            .get("full_text")
            .map(|it| it.as_str().unwrap_or_default())
            .unwrap_or_default();
        let is_rt = full_text.starts_with("RT @");
        print!(".");
        conn.execute("insert into tweets (id, is_rt) values (?1, ?2)", &[id, &is_rt.to_string()])?;
    }

    Ok(())
}
