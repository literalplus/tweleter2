# tweleter2

This app can delete your tweets using the GraphQL API provided by Twitter.

To set it up, copy the necessary values from your browser dev tools into `.env`.

For the import, remove the weird JS stuff from the first line of `tweets.js` so that it becomes valid JSON.

Then: `cargo run -- import PATH-TO-TWEETS-JS`

To delete a batch of tweets: `cargo run -- delete-some`

Please note that the Twitter API has some rate limits, so you need to do this more often.
