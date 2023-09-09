use std::{
    collections::HashSet,
    io::Read,
    time::Instant,
};

use anyhow::*;
use clap::Args;
use curl::easy::{Easy, List};
use log::{error, warn};
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

    #[arg(long, env = "EXEMPT_TWEET_IDS")]
    exempt_tweet_ids: Vec<String>,
    #[arg(long, env = "TWEET_LIMIT", default_value = "1000")]
    tweet_limit: u64,
}

#[derive(Debug)]
struct TweetDat {
    id: String,
}

pub fn run(params: Params) -> Result<()> {
    let conn = Connection::open("tweets.db")?;

    let mut stmt = conn
        .prepare("SELECT t.id, t.is_rt FROM tweets t WHERE is_rt = 'false' LIMIT ?1")
        .expect("prepare select");

    let res = stmt
        .query_map([params.tweet_limit], |row| {
            Result::Ok(TweetDat {
                id: row.get(0).expect("id"),
            })
        })
        .expect("extract result");

    let exempt_tweet_ids: HashSet<String> = params
        .exempt_tweet_ids
        .iter()
        .map(|it| it.to_owned())
        .collect();

    for (i, tweet) in res.enumerate() {
        let start = Instant::now();
        let tweet = tweet?;
        if exempt_tweet_ids.contains(&tweet.id) {
            warn!("Skipped deleting {:?} due to exemption.", tweet);
        } else {
            if let Err(e) = curl_it(&params, &tweet, i) {
                println!();
                let err_str = format!("{:?}", e);
                error!("Error during request: {}", err_str);
                if err_str.contains("[28] Timeout was reached (Connection timed out after 3000") { // apparently the error message is sometimes off by a few milliseconds
                    // Twitter infra does this every like 10 minutes, retry once after delay
                    warn!("Looks like timeout, retry after 10 seconds...");
                    std::thread::sleep(Duration::from_secs(10));
                    curl_it(&params, &tweet, i).context("second attempt")?;
                } else {
                    error!("Unknown type of error, not retrying.");
                    return Err(e);
                }
            }
        }
        conn.execute("DELETE FROM tweets where id=?1", &[&tweet.id])
            .context("deleting it")?;

        // Rumour has it that the limit is 900 calls per 15 minutes = 1 tweet per second
        let time_to_sleep = 1010_u128.saturating_sub(start.elapsed().as_millis()).clamp(100, 1500) as u64;
        std::thread::sleep(Duration::from_millis(time_to_sleep));
    }

    let res = conn
        .query_row("SELECT COUNT(*) FROM tweets WHERE is_rt = 'false'", [], |row| {
            Result::Ok(row.get::<usize, u64>(0).expect("count"))
        })
        .expect("getting count");
    println!("{} (non-RT) tweets left.", res);

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

    // Sometimes, Twitter just randomly blocks forever or so
    curl.timeout(Duration::from_secs(30))?;

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
        transfer.perform()?;
    }

    Ok(response_body)
}
