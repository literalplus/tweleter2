#!/bin/bash

# ./curl.sh CSRF AUTH MULTI ID

curl 'https://twitter.com/i/api/graphql/VaenaVgh5q5ih7kvyVjgtg/DeleteTweet' -X POST -H 'Accept: */*' -H 'Accept-Language: en-GB,en;q=0.7,de;q=0.3' -H 'Content-Type: application/json' -H 'x-csrf-token: '$1 -H 'authorization: Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA' -H 'Cookie: auth_token='$2'; auth_multi="'$3'"; ct0='$1 --data-raw '{"variables":{"tweet_id":"'$4'","dark_request":false},"queryId":"VaenaVgh5q5ih7kvyVjgtg"}' -vvv