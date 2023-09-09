use std::time::Duration;

use anyhow::*;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use flexi_logger::{colored_default_format, detailed_format, Logger, LoggerHandle, WriteMode};
use human_panic::setup_panic;
use import::run;
use log::{debug, warn, Level};
use rusqlite::{Connection, Result as SqlResult};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Import(ImportParams),
    DeleteOne(run_delete::Params),
}

#[derive(Args)]
pub struct ImportParams {
    file: std::path::PathBuf,
}

fn main() -> Result<()> {
    setup_panic!();
    if let Err(env_err) = dotenvy::dotenv() {
        if env_err.not_found() {
            warn!("No `.env` file found (recursively). You usually want to have one.")
        } else {
            return Err(env_err).with_context(|| "Failed to load `.env` file");
        }
    }
    let cli = Cli::parse();
    let logger = configure_log_from(&cli)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .with_context(|| "Failed to start Tokio runtime")?;
    let _guard = runtime.enter();

    let res = do_start(cli);

    debug!("Waiting up to 15 seconds for remaining tasks to finish");
    runtime.shutdown_timeout(Duration::from_secs(15));

    // Important with non-direct write mode
    // Handle needs to be kept alive until end of program
    logger.flush();

    res
}

fn configure_log_from(params: &Cli) -> Result<LoggerHandle> {
    // log_level() returns None iff verbosity < 0, i.e. being most quiet seems reasonable
    let cli_level = params.verbose.log_level().unwrap_or(Level::Error);

    let log_builder = Logger::try_with_env_or_str(cli_level.to_string())
        .context("Failed to parse logger spec from env RUST_LOG or cli level")?
        .write_mode(WriteMode::Async)
        .format_for_stdout(colored_default_format)
        .format_for_files(detailed_format);

    log_builder
        .start()
        .context("Failed to start logger handle w/o specfile")
}

fn do_start(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Import(it) => import::run(it),
        Commands::DeleteOne(it) => run_delete::run(it),
    }
}

mod import;
mod run_delete {
    use std::{io::{stdout, Read, Write}, process::{Command, Stdio}, borrow::Cow};

    use anyhow::*;
    use clap::Args;
    use curl::easy::{Easy, List};
    use log::{log_enabled, debug, warn};
    use rusqlite::Connection;

    #[derive(Args)]
    pub struct Params {
        #[arg(long, env = "BEARER_TOKEN")]
        bearer_token: String,
        #[arg(long, env = "AUTH_MULTI")]
        auth_multi: String,
        #[arg(long, env = "AUTH_TOKEN")]
        auth_token: String,
    }

    #[derive(Debug)]
    struct TweetDat {
        id: String,
    }

    pub fn run(params: Params) -> Result<()> {
        let conn = Connection::open("tweets.db")?;

        let mut stmt = conn
            .prepare("SELECT t.id, t.is_rt FROM tweets t WHERE is_rt = 'false' LIMIT 1")
            .expect("prepare select");

        let res = stmt
            .query_map([], |row| {
                Result::Ok(TweetDat {
                    id: row.get(0).expect("id"),
                })
            })
            .expect("extract result");

        for tweet in res {
            cmd_it2(&params, tweet?)?;
        }

        Ok(())
    }

    fn curl_it(params: &Params, tweet: TweetDat) -> Result<()> {
        print!("{:?}", tweet);
        let req_id = "VaenaVgh5q5ih7kvyVjgtg";
        let mut curl = Easy::new();
        let url = format!(
            "https://twitter.com/i/api/graphql/{}/DeleteTweet",
            //"https://test.apps.nowak.cloud/i/api/graphql/{}/DeleteTweet",
            req_id
        );
        println!("{}", url);
        curl.url(&url)?;
        curl.post(true)?;

        let mut headers = List::new();
        headers.append(
            "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/116.0",
        )?;
        headers.append("Accept: */*")?;
        headers.append("Accept-Language: en-GB,en;q=0.7,de;q=0.3")?;
        headers.append("Content-Type: application/json")?;

        // format matters here apparently
        let csrf_token = "b21f14183de9d4dd11ae119b8ca68cbd5f0b1f6528615869b209846a2aaded45f76fc09d3b9c51287c78f5f7317854388710de56ed5511778d88db7ebdabca77dfb58e64d88b9a3e3cad1ad539658dbc";
        let csrf = format!("x-csrf-token: {}", csrf_token);
        println!("{}", csrf);
        headers.append(&csrf)?;
        let auth = format!("authorization: Bearer {}", params.bearer_token);
        println!("{}", auth);
        headers.append(&auth)?;
        let cookie = format!(
            "Cookie: auth_token={}; auth_multi=\"{}\"; ct0={}",
            params.auth_token, params.auth_multi, csrf_token
        );
        println!("{}", cookie);
        headers.append(&cookie)?;
        curl.http_headers(headers).expect("set headers");

        let dart = format!(
            "{{\"variables\":{{\"tweet_id\":\"{}\",\"dark_request\":false}},\"queryId\":\"{}\"}}",
            tweet.id, req_id
        );
        println!("dart is {}", dart);
        let mut data = dart.as_bytes();
        curl.post_field_size(data.len() as u64)?;

        let mut transfer = curl.transfer();
        transfer
            .read_function(|buf| Result::Ok(data.read(buf).unwrap_or(0)))
            .expect("read");
        transfer
            .write_function(|dat| {
                stdout().write_all(dat).unwrap();
                Result::Ok(dat.len())
            })
            .expect("write to work");
        transfer.perform().expect("it to perform");
        Ok(())
    }

    fn cmd_it(params: &Params, tweet: TweetDat) -> Result<()> {
        print!("{:?}", tweet);
        let req_id = "VaenaVgh5q5ih7kvyVjgtg";
        let mut cmd = Command::new("curl");
        let url = format!(
            //"https://twitter.com/i/api/graphql/{}/DeleteTweet",
            "https://test.apps.nowak.cloud/i/api/graphql/{}/DeleteTweet",
            req_id
        );
        println!("{}", url);
        cmd.arg(url).arg("-X").arg("POST");

        let mut headers = vec![];
        // headers.push(
        //     "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/116.0",
        // );
        headers.push("Accept: */*");
        headers.push("Accept-Language: en-GB,en;q=0.7,de;q=0.3");
        headers.push("Content-Type: application/json");

        // format matters here apparently
        let csrf_token = "b21f14183de9d4dd11ae119b8ca68cbd5f0b1f6528615869b209846a2aaded45f76fc09d3b9c51287c78f5f7317854388710de56ed5511778d88db7ebdabca77dfb58e64d88b9a3e3cad1ad539658dbc";
        let csrf = format!("x-csrf-token: {}", csrf_token);
        println!("{}", csrf);
        headers.push(&csrf);
        let auth = format!("authorization: Bearer {}", params.bearer_token);
        println!("{}", auth);
        headers.push(&auth);
        let cookie = format!(
            "Cookie: auth_token={}; auth_multi=\"{}\"; ct0={}",
            params.auth_token, params.auth_multi, csrf_token
        );
        println!("{}", cookie);
        headers.push(&cookie);

        for header in headers {
            println!(" -> {}", header);
            cmd.arg("-H").arg(header);
        }

        let dart = format!(
            "{{\"variables\":{{\"tweet_id\":\"{}\",\"dark_request\":false}},\"queryId\":\"{}\"}}",
            tweet.id, req_id
        );
        println!("dart is {}", dart);
        cmd.arg("--data-raw").arg(dart);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        if log_enabled!(log::Level::Debug) || true {
            let args: Vec<Cow<'_, str>> = cmd
                .get_args()
                .map(|os_str| os_str.to_string_lossy())
                .collect();
            warn!("Calling with arguments: {}", args.join(" "));
        }

        let mut child = cmd.spawn().context("spawn")?;
        let exit = child.wait().context("wait")?;

        println!();

        if exit.success() {
            println!("Seems like it worked!");
            Ok(())
        } else {
            println!("oh no! {}", exit);
            bail!("command execution failed with {}", exit);
        }
    }

    fn cmd_it2(params: &Params, tweet: TweetDat) -> Result<()> {
        print!("{:?}", tweet);
        let mut cmd = Command::new("bash");
        cmd.arg("./curl.sh");

        // format matters here apparently
        let csrf_token = "b275c5e53f8dd327471722e8efd0a2cab78b9dcb284de04526241a2a317700a9a7a9b4fe1f02b42d0f80bdd0b5162a33f0493c8c18dfaa3e1e05fbde659ea8d79dedb582c48b38771d24fad70f47dd98";
        cmd.arg(csrf_token);
        cmd.arg(&params.auth_token);
        cmd.arg(&params.auth_multi);
        cmd.arg(tweet.id);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        if log_enabled!(log::Level::Debug) || true {
            let args: Vec<Cow<'_, str>> = cmd
                .get_args()
                .map(|os_str| os_str.to_string_lossy())
                .collect();
            warn!("Calling with arguments: {}", args.join(" "));
        }

        let mut child = cmd.spawn().context("spawn")?;
        let exit = child.wait().context("wait")?;

        println!();

        if exit.success() {
            println!("Seems like it worked!");
            Ok(())
        } else {
            println!("oh no! {}", exit);
            bail!("command execution failed with {}", exit);
        }
    }
}
