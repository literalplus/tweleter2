use anyhow::*;
use clap::Args;
use rusqlite::{Connection, Row};

#[derive(Args)]
pub struct Params {}

pub fn run(_params: Params) -> Result<()> {
    let conn = Connection::open("tweets.db")?;

    let mut stmt = conn.prepare("SELECT COUNT(*) FROM tweets WHERE is_rt = ?1")?;

    let mapper = |row: &Row| Result::Ok(row.get::<usize, u64>(0).expect("count"));

    let no_rt = stmt
        .query_row(&[&false.to_string()], mapper)
        .context("query no-rt")?;
    let only_rt = stmt
        .query_row(&[&true.to_string()], mapper)
        .context("query only-rt")?;
    let sum = no_rt + only_rt;

    println!(" --- {} tweets left to delete, of these:", sum);
    println!("  * {} true tweets", no_rt);
    println!("  * {} retweets", only_rt);

    Ok(())
}
