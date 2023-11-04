# tweleter2

This app can delete your tweets using the GraphQL API provided by Twitter.

To set it up:
 1. download your Twitter archive
 2. copy `.env.template` to `.env`
 3. insert the necessary values from your browser dev tools into `.env`. the easiest way to do this is to manually delete a tweet in the UI and copy the values that were sent with the request. where to get them is documented in `.env.template`.
 4. configure popularity thresholds (`EXEMPT_*`) of tweets to keep
 5. remove the weird JS stuff from the first line of `tweets.js` in the archive, so that it becomes valid JSON.
 6. import the tweets into the SQLite DB: `cargo run --release -- import PATH-TO-TWEETS-JS`

To delete a batch of tweets: `cargo run --release -- delete-some --tweet-limit=10000`

# Disclaimer

The sending rate is designed not to hit Twitter's rate limits at the time of creation (September 2023), but it is your reponsibility to ensure that this is still the case and that your usage of this tool is compliant with the latest version of Twitter's Terms of Service. Also, it may be that Twitter changes or removes the API we're using.
