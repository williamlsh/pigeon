use crate::client::ReusableBlockingClient;
use log::{debug, error, info, trace, warn};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone)]
pub struct UrlBuilder(Url);

impl UrlBuilder {
    pub fn new(base_url: &Url, user_id: &str) -> Self {
        let url = Url::options()
            .base_url(Some(base_url))
            .parse(format!("users/{}/tweets", user_id).as_str())
            .expect("could not parse url form user_id segment");
        Self(url)
    }

    pub fn tweet_fields(mut self, tweet_fields: Vec<&str>) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("tweet.fields", &tweet_fields.join(","));
        self
    }

    pub fn expansions(mut self, expansions: Vec<&str>) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("expansions", &expansions.join(","));
        self
    }

    pub fn media_fields(mut self, media_fields: Vec<&str>) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("media.fields", &media_fields.join(","));
        self
    }

    pub fn max_results(mut self, max_results: u8) -> Self {
        self.0
            .query_pairs_mut()
            .append_pair("max_results", &max_results.to_string());
        self
    }

    pub fn build(self) -> Url {
        self.0
    }
}

/// A PaginatedTimeline only supports one twitter user's timeline.
#[derive(Debug)]
pub struct PaginatedTimeline<'a> {
    client: &'a ReusableBlockingClient,
    url: Url,
    pagination_token: Option<String>,
    page: usize,
}

impl<'a> PaginatedTimeline<'a> {
    pub fn new(
        client: &'a ReusableBlockingClient,
        url: Url,
        pagination_token: Option<String>,
    ) -> Self {
        Self {
            client,
            url,
            pagination_token,
            page: 0,
        }
    }

    fn try_next(&mut self) -> Result<Option<Timeline>, String> {
        // Check if pagination token is present.
        let url = match self.pagination_token.take() {
            Some(pagination_token) => self.pagination_token(&pagination_token),
            None => match self.page {
                // The first request.
                0 => self.url.clone(),
                // The last request.
                n => {
                    info!("All pages are done, total pages: {}", n);
                    return Ok(None);
                }
            },
        };
        debug!("current url: {}", url.as_str());

        // Check response status.
        let response = self
            .client
            .get(&url)
            .map_err(|error| format!("get request failed: {:?}", error))?;
        match response.status() {
            StatusCode::OK => {
                let timeline: Timeline = response
                    .json()
                    .map_err(|error| format!("could not deserialize json response: {:?}", error))?;
                trace!("got timeline: {:?}", timeline);

                // Keep the pagination token for next request.
                match timeline.meta.clone() {
                    Some(meta) => self.pagination_token = meta.next_token,
                    None => return Ok(None),
                }

                // Increase page number on request success.
                self.page += 1;
                Ok(Some(timeline))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                info!(
                    "twitter timeline endpoint rate limit reached, please wait for at least 15 mins: {}",
                    response.status()
                );
                Ok(None)
            }
            x => {
                warn!("request not successful, got response status: {}", x);
                Ok(None)
            }
        }
    }

    // No unit test for this function.
    /// Since Url.append_pair will append duplicated key value pairs,
    /// so we don't mutate original Url, we return a clone one.
    fn pagination_token(&self, pagination_token: &str) -> Url {
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("pagination_token", pagination_token);
        url
    }
}

impl<'a> Iterator for PaginatedTimeline<'a> {
    type Item = Timeline;

    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            Ok(timeline) => timeline,
            Err(err) => {
                error!(
                    "an error occurred when iterating paginated timeline: {}",
                    err
                );
                None
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Timeline {
    data: Option<Vec<Data>>,
    includes: Option<Includes>,
    meta: Option<Meta>,
}

impl Timeline {
    pub fn next_token(&self) -> Option<String> {
        match &self.meta {
            Some(meta) => meta.next_token.clone(),
            None => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Data {
    id: String,
    created_at: String,
    attachments: Option<Attachments>,
    text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Attachments {
    media_keys: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Includes {
    media: Vec<Media>,
    tweets: Option<Vec<Data>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Media {
    width: Option<u16>,
    media_key: String,
    height: Option<u16>,
    #[serde(rename = "type")]
    typ: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Meta {
    oldest_id: String,
    newest_id: String,
    result_count: usize,
    next_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Timeline, UrlBuilder};
    use crate::twitter::API_ENDPOINT_BASE;
    use url::Url;

    const USER_ID: &str = "abc";

    #[test]
    fn test_new_url() {
        let base_url = Url::parse(API_ENDPOINT_BASE).unwrap();
        let url = UrlBuilder::new(&base_url, USER_ID);

        assert_eq!(
            format!("{}users/{}/tweets", API_ENDPOINT_BASE, USER_ID),
            url.0.as_str()
        );
    }

    #[test]
    fn test_url_queries() {
        let base_url = Url::parse(API_ENDPOINT_BASE).unwrap();
        let url = UrlBuilder::new(&base_url, USER_ID)
            .tweet_fields(vec!["a", "b", "c"])
            .expansions(vec!["a", "b", "c"])
            .media_fields(vec!["a", "b", "c"])
            .max_results(100);

        assert_eq!(
            "tweet.fields=a%2Cb%2Cc&expansions=a%2Cb%2Cc&media.fields=a%2Cb%2Cc&max_results=100",
            url.0.query().unwrap()
        );
    }

    #[test]
    fn test_parse_timeline() {
        let timeline_data = r#"
        {
          "data": [
            {
              "text": "Learn how the municipality of The Hague was able to create an improved experience on their roads via social media monitoring  — through their partnership with PublicSonar. \n\nRead about it here: https://t.co/9Ex9oas1kO https://t.co/mJTz4ckm6c",
              "created_at": "2022-02-28T18:20:00.000Z",
              "id": "1498362363132747780",
              "attachments": {
                "media_keys": [
                  "3_1496550979738685440"
                ]
              }
            },
            {
              "text": "Bay area, don't forget tomorrow is the first #TwitterDevConnect meetup of 2022! We have a few spots left so RSVP now: https://t.co/AahQG8MzCK https://t.co/5ah3tsCjNJ",
              "created_at": "2022-02-23T22:45:00.000Z",
              "id": "1496617113133494273",
              "attachments": {
                "media_keys": [
                  "3_1496540189753176064"
                ]
              }
            },
            {
              "text": "Raw data to insights in a matter of minutes.⏱ Introducing the Twitter API toolkit for Google Cloud: access the brand new guide to easily process, analyze, and visualize massive amounts of Tweets today.👇 #BuildWhatsNext\n\nhttps://t.co/jEXo0X6Fsp",
              "created_at": "2022-02-22T19:58:48.000Z",
              "id": "1496212901383884805"
            },
            {
              "text": "Join @ashevat, @jessicagarson and @alanbenlee Thursday 2/24 at 3:05pm PT for this month’s town hall conversation on the recent updates to the Twitter Developer Platform. https://t.co/aJ7yayEwbx",
              "created_at": "2022-02-21T18:24:39.000Z",
              "id": "1495826817638633472"
            },
            {
              "text": "We want to meet you! If you are in the Bay Area, join us for a #TwitterDev Connect meetup on February 24! RSVP here: https://t.co/AahQG8uYea",
              "created_at": "2022-02-18T17:23:14.000Z",
              "id": "1494724197657899008"
            },
            {
              "text": "New ways to build discovery and analytics tools for #TwitterSpaces. Learn more about our new endpoint that returns Tweets from a Space, plus the new subcriber_count field. 👀  ⬇️  #BuildWhatsNext\n\nhttps://t.co/Ev3Stajmjl",
              "created_at": "2022-02-17T18:58:45.000Z",
              "id": "1494385850649436160"
            },
            {
              "text": "Celebrate the #GoodBots! Today, automated account labels are available to all developer-created bot accounts. https://t.co/GyT7Duh9Yu",
              "created_at": "2022-02-16T20:12:20.000Z",
              "referenced_tweets": [
                {
                  "type": "quoted",
                  "id": "1494040671048581123"
                }
              ],
              "id": "1494041977972744200"
            },
            {
              "text": "Stay relevant! The sort_order parameter now allows you to return Tweets based on relevancy when using the search endpoints on the Twitter API v2. #BuildWhatsNext https://t.co/ULOQlTXrqd",
              "created_at": "2022-02-09T19:03:07.000Z",
              "id": "1491487846623956993"
            },
            {
              "text": "We've created a place where people on Twitter can find and get started with ready-to-use Twitter solutions from our developer community. 🔍 https://t.co/WV8sBHDxa1",
              "created_at": "2022-02-02T17:41:27.000Z",
              "referenced_tweets": [
                {
                  "type": "quoted",
                  "id": "1488619414584922116"
                }
              ],
              "id": "1488930580054085633"
            },
            {
              "text": "Interested in learning how to make bots with the Twitter API? @JessicaGarson's tutorial walks you through her latest bot @FactualCat, which Tweets out cat facts daily. #BuildWhatsNext \n\nhttps://t.co/RBEbZCmNNq",
              "created_at": "2022-02-01T20:30:09.000Z",
              "id": "1488610643766824961"
            }
          ],
          "includes": {
            "media": [
              {
                "url": "https://pbs.twimg.com/media/FMTRL9-WUAA9RMf.png",
                "media_key": "3_1496550979738685440",
                "type": "photo",
                "height": 205,
                "width": 253
              },
              {
                "url": "https://pbs.twimg.com/media/FMTHX6JVcAAC2dv.jpg",
                "media_key": "3_1496540189753176064",
                "type": "photo",
                "height": 900,
                "width": 1600
              }
            ],
            "tweets": [
              {
                "text": "Get your bots in here! Remember when we chatted about all things, #GoodBots? Well now we are celebrating the bots who make a positive contribution to Twitter, all over the world. https://t.co/e1OqJjRZiG",
                "created_at": "2022-02-16T20:07:08.000Z",
                "id": "1494040671048581123",
                "attachments": {
                  "media_keys": [
                    "16_1494039614381862912"
                  ]
                }
              },
              {
                "text": "Put the NEW Twitter Toolbox to work for you. These ready-to-use tools are low-cost and built by our developer community to help you get even more out of Twitter.",
                "created_at": "2022-02-01T21:05:00.000Z",
                "id": "1488619414584922116"
              }
            ]
          },
          "meta": {
            "oldest_id": "1488610643766824961",
            "newest_id": "1498362363132747780",
            "result_count": 10,
            "next_token": "7140dibdnow9c7btw3z45ddzr2fig4a4y9q0vs4alejap"
          }
        }"#;

        serde_json::from_str::<Timeline>(timeline_data).unwrap();
    }
}
