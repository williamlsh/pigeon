use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{info, trace, warn};
use url::Url;

use super::API_ENDPOINT_BASE;

/// Timeline continually yields all tweets in timeline which may be paginated.
pub(crate) struct Timeline<'a> {
    client: &'a Client,
    url: Url,
    auth_token: &'a str,
    pagination_token: Option<PaginationToken>,
    page: u8,
    texts: <Vec<Data> as IntoIterator>::IntoIter,
}

#[derive(Debug)]
pub(crate) enum PaginationToken {
    NextToken(String),
    TweetID(String),
}

impl<'a> Timeline<'a> {
    pub(crate) fn new(
        client: &'a Client,
        url: Url,
        auth_token: &'a str,
        pagination_token: Option<PaginationToken>,
    ) -> Self {
        Self {
            client,
            url,
            auth_token,
            pagination_token,
            page: 0,
            texts: vec![].into_iter(),
        }
    }

    pub(crate) async fn try_next(&mut self) -> Result<Option<Data>> {
        if let Some(text) = self.texts.next() {
            return Ok(Some(text));
        }

        // Check if pagination token is present.
        let url = match self.pagination_token.take() {
            Some(pagination_token) => self.url_with_pagination(pagination_token),
            None => match self.page {
                // The first request.
                0 => self.url.clone(),
                // The last request.
                n => {
                    info!("Finished polling timeline, total pages: {}", n);
                    return Ok(None);
                }
            },
        };

        let response = self
            .client
            .get(url)
            .bearer_auth(self.auth_token)
            .send()
            .await
            .with_context(|| "Failed to request timeline")?;
        // Check response status.
        match response.status() {
            StatusCode::OK => {
                let mut timeline: Tweets = response
                    .json()
                    .await
                    .with_context(|| "Failed to deserialize json response")?;
                trace!("got timeline: {:?}", timeline);

                // Keep the pagination token for next request.
                self.pagination_token = timeline
                    .meta
                    .next_token
                    .take()
                    .map(PaginationToken::NextToken);

                // Increase page number on request success.
                match timeline.data {
                    Some(tweets) => {
                        self.page += 1;
                        self.texts = tweets.into_iter();
                        Ok(self.texts.next())
                    }
                    // In a case that "start_time" query parameter is specified in timeline request,
                    // "next_token" is always returned in the last page metadata. To avoid endless unnecessary
                    // page requests, we exit immediately here.
                    None => Ok(None),
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                info!(
                "twitter timeline endpoint rate limit reached, please wait for at least 15 mins before next try: {}",
                response.status());
                Ok(None)
            }
            x => {
                warn!(
                    "request not successful, got response status: {} and body: {}",
                    x,
                    response.text().await.unwrap_or_else(|_| "".to_string())
                );
                Ok(None)
            }
        }
    }

    /// We don't mutate original `Url`, we return a clone one since `Url.append_pair` will append duplicated key value pairs.
    fn url_with_pagination(&self, pagination_token: PaginationToken) -> Url {
        let mut url = self.url.clone();
        match pagination_token {
            PaginationToken::NextToken(next_token) => url
                .query_pairs_mut()
                .append_pair("pagination_token", &next_token),
            PaginationToken::TweetID(tweet_id) => {
                url.query_pairs_mut().append_pair("since_id", &tweet_id)
            }
        };
        url
    }
}

/// Response from Twitter timeline api.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Tweets {
    data: Option<Vec<Data>>,
    meta: Meta,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct Data {
    pub(crate) id: String,
    pub(crate) created_at: String,
    pub(crate) text: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Meta {
    oldest_id: Option<String>,
    newest_id: Option<String>,
    result_count: Option<u8>,
    next_token: Option<String>,
}

// Builds a Twitter user timeline endpoint URL.
#[derive(Debug, Clone)]
pub(crate) struct UrlBuilder(Url);

impl UrlBuilder {
    pub(crate) fn new(user_id: &str) -> Result<Self> {
        let base_url = Url::parse(API_ENDPOINT_BASE).unwrap();
        Url::options()
            .base_url(Some(&base_url))
            .parse(format!("users/{}/tweets", user_id).as_str())
            .map(Self)
            .with_context(|| "Failed to parse url from user_id segment")
    }

    pub(crate) fn tweet_fields(mut self, tweet_fields: Vec<&str>) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("tweet.fields", &tweet_fields.join(","));
        self
    }

    pub(crate) fn max_results(mut self, max_results: u8) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("max_results", &max_results.to_string());
        self
    }

    /// String format for `start_time` is RFC3339, for example, "2020-12-12T01:00:00Z".
    pub(crate) fn start_time(mut self, start_time: Option<&str>) -> Self {
        if let Some(start_time) = start_time {
            self.0
                .query_pairs_mut()
                .append_pair("start_time", start_time);
        }
        self
    }

    /// String format for `end_time` is RFC3339, for example, "2020-12-12T01:00:00Z".
    pub(crate) fn end_time(mut self, end_time: Option<&str>) -> Self {
        if let Some(end_time) = end_time {
            self.0.query_pairs_mut().append_pair("end_time", end_time);
        }
        self
    }

    pub(crate) fn build(self) -> Url {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use tracing::debug;

    use super::{PaginationToken, Timeline, Tweets, UrlBuilder, API_ENDPOINT_BASE};

    #[test]
    fn build_url() {
        let url = UrlBuilder::new("123").unwrap().build();
        assert_eq!(
            format!("{}users/{}/tweets", API_ENDPOINT_BASE, "123"),
            url.as_str()
        );
    }

    #[test]
    fn url_queries() {
        let url = UrlBuilder::new("")
            .unwrap()
            .tweet_fields(vec!["created_at"])
            .max_results(100)
            .start_time(Some("2022-11-21T12:23:43.812Z"))
            .end_time(Some("2022-11-24T12:23:43.812Z"))
            .build();
        assert_eq!(
          "tweet.fields=created_at&max_results=100&start_time=2022-11-21T12%3A23%3A43.812Z&end_time=2022-11-24T12%3A23%3A43.812Z",
            url.query().unwrap()
        );
    }

    #[test]
    fn parse_timeline() {
        let timeline_data = r#"
        {
          "data": [
            {
              "created_at": "2022-11-02T23:15:29.000Z",
              "text": "As always, we’re just a Tweet away, so feel free to reach out with any questions. We’re grateful for your partnership to #BuildWhatsNext",
              "id": "1587946527955329024",
              "edit_history_tweet_ids": [
                "1587946527955329024"
              ]
            },
            {
              "created_at": "2022-11-02T23:15:29.000Z",
              "text": "We’ll still celebrate the soon-to-be-announced winners of our Chirp Developer Challenge - stay tuned for more details!",
              "id": "1587946526617264128",
              "edit_history_tweet_ids": [
                "1587946526617264128"
              ]
            },
            {
              "created_at": "2022-11-02T23:15:28.000Z",
              "text": "We’re currently hard at work to make Twitter better for everyone, including developers! We’ve decided to cancel the #Chirp developer conference while we build some things that we’re excited to share with you soon.",
              "id": "1587946525245816832",
              "edit_history_tweet_ids": [
                "1587946525245816832"
              ]
            },
            {
              "created_at": "2022-11-01T19:00:00.000Z",
              "text": "💡 #TipTuesday:  Ever wondered how to get the video URL from a Tweet in Twitter API v2? 👀 Here’s a walkthrough, using our TypeScript SDK. 💫\n\nhttps://t.co/tFQ4Eskq7t",
              "id": "1587519847281397767",
              "edit_history_tweet_ids": [
                "1587519847281397767"
              ]
            },
            {
              "created_at": "2022-10-31T13:00:01.000Z",
              "text": "✍️Fill in the blank ⬇️\n\nI start my morning off by _____",
              "id": "1587066866824085505",
              "edit_history_tweet_ids": [
                "1587066866824085505"
              ]
            }
          ],
          "meta": {
            "result_count": 5,
            "newest_id": "1587946527955329024",
            "oldest_id": "1587066866824085505",
            "next_token": "7140dibdnow9c7btw423x78o50g6e358t5r7iusluud6d"
          }
        }"#;

        serde_json::from_str::<Tweets>(timeline_data).unwrap();
    }

    // To test this function:
    // RUST_LOG=debug cargo test tweets -- --ignored '[auth_token]'
    #[test_log::test(tokio::test)]
    #[ignore = "require command line input"]
    async fn tweets() {
        let mut args = std::env::args().rev();
        let auth_token = args.next().unwrap();

        let client = Client::new();
        let endpoint = UrlBuilder::new("2244994945")
            .unwrap()
            .tweet_fields(vec!["created_at"])
            .max_results(10)
            .start_time(Some("2022-10-25T00:00:00.000Z"))
            .end_time(Some("2022-11-04T00:00:00.000Z"))
            .build();
        {
            debug!("Timeline without pagination token");
            let mut timeline = Timeline::new(&client, endpoint.clone(), &auth_token, None);

            while let Some(tweet) = timeline.try_next().await.unwrap() {
                debug!("{:#?}", tweet);
            }
        }
        {
            debug!("Timeline with pagination token");
            let tweet_id = PaginationToken::TweetID("1586025008899448832".into());
            let mut timeline = Timeline::new(&client, endpoint, &auth_token, Some(tweet_id));
            while let Some(tweet) = timeline.try_next().await.unwrap() {
                debug!("{:#?}", tweet);
            }
        }
    }
}
