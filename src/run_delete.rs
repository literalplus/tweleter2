use std::io::Read;

use anyhow::*;
use clap::Args;
use curl::easy::{Easy, List};
use log::error;
use rusqlite::Connection;
use std::time::Duration;

#[derive(Args)]
pub struct Params {
    #[arg(long, env = "BEARER_TOKEN")]
    bearer_token: String,
    #[arg(long, env = "AUTH_MULTI")]
    auth_multi: String,
    #[arg(long, env = "AUTH_TOKEN")]
    auth_token: String,
    #[arg(long, env = "CSRF_TOKEN")]
    csrf_token: String,
}

#[derive(Debug)]
struct TweetDat {
    id: String,
}

pub fn run(params: Params) -> Result<()> {
    let conn = Connection::open("tweets.db")?;

    // Rumour has it that the limit is 900 calls per 15 minutes

    let mut stmt = conn
        .prepare("SELECT t.id, t.is_rt FROM tweets t WHERE is_rt = 'false' LIMIT 100")
        .expect("prepare select");

    let res = stmt
        .query_map([], |row| {
            Result::Ok(TweetDat {
                id: row.get(0).expect("id"),
            })
        })
        .expect("extract result");

    for (i, tweet) in res.enumerate() {
        let tweet = tweet?;
        curl_it(&params, &tweet, i)?;
        conn.execute("DELETE FROM tweets where id=?1", &[&tweet.id])
            .context("deleting it")?;
        std::thread::sleep(Duration::from_millis(500));
    }

    let res = conn
        .query_row("SELECT COUNT(*) FROM tweets", [], |row| {
            Result::Ok(row.get::<usize, u64>(0).expect("count"))
        })
        .expect("getting count");
    println!("{} tweets left.", res);

    Ok(())
}

fn curl_it(params: &Params, tweet: &TweetDat, i: usize) -> Result<()> {
    print!(" [{: >4}]={} -> ", i, tweet.id);
    let req_id = "VaenaVgh5q5ih7kvyVjgtg";
    let mut curl = Easy::new();
    let url = format!(
        "https://twitter.com/i/api/graphql/{}/DeleteTweet",
        //"https://test.apps.nowak.cloud/i/api/graphql/{}/DeleteTweet",
        req_id
    );
    curl.url(&url)?;
    curl.post(true)?;

    set_headers(&mut curl, params).context("setting headers")?;

    let data = format!(
        "{{\"variables\":{{\"tweet_id\":\"{}\",\"dark_request\":false}},\"queryId\":\"{}\"}}",
        tweet.id, req_id
    );

    let response_bytes = do_transfer(curl, data.as_bytes())?;

    let response_str =
        String::from_utf8(response_bytes).context("response body utf8 conversion")?;
    println!("{}", response_str);

    if response_str != "{\"data\":{\"delete_tweet\":{\"tweet_results\":{}}}}" {
        error!("This is an unexpected response body!");
        bail!("response body not as expected");
    }

    Ok(())
}

fn set_headers(curl: &mut Easy, params: &Params) -> Result<()> {
    let mut headers = List::new();
    headers.append(
        "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/116.0",
    )?;
    headers.append("Accept: */*")?;
    headers.append("Accept-Language: en-GB,en;q=0.7,de;q=0.3")?;
    headers.append("Content-Type: application/json")?;

    // format matters here apparently
    let csrf_token = &params.csrf_token;
    let csrf = format!("x-csrf-token: {}", csrf_token);
    headers.append(&csrf)?;
    let auth = format!("authorization: Bearer {}", params.bearer_token);
    headers.append(&auth)?;
    let cookie = format!(
        "Cookie: auth_token={}; auth_multi=\"{}\"; ct0={}",
        params.auth_token, params.auth_multi, csrf_token
    );
    headers.append(&cookie)?;
    curl.http_headers(headers).expect("set headers");
    Ok(())
}

fn do_transfer(mut curl: Easy, mut request_body: &[u8]) -> Result<Vec<u8>> {
    let mut response_body = vec![];
    curl.post_field_size(request_body.len() as u64)?;

    {
        let mut transfer = curl.transfer();
        transfer
            .read_function(|buf| Result::Ok(request_body.read(buf).unwrap_or(0)))
            .expect("read");
        transfer
            .write_function(|dat| {
                response_body.extend_from_slice(dat);
                Result::Ok(dat.len())
            })
            .expect("write to work");
        transfer.perform().expect("it to perform");
    }

    Ok(response_body)
}
