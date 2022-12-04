use anyhow::{Context, Result};
use log::warn;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

use super::API_ENDPOINT_BASE;

/// Response from Twitter users lookup api.
#[derive(Debug, Deserialize)]
pub(crate) struct Users {
    data: Option<Vec<Data>>,
    errors: Option<Vec<Error>>,
}

#[derive(Debug, Deserialize)]
struct Data {
    id: String,
    name: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct Error {
    value: String,
    detail: String,
    title: String,
    resource_type: String,
    parameter: String,
    resource_id: String,
    #[serde(rename(deserialize = "type"))]
    typ: String,
}

impl Users {
    /// Fetch users to return a username to user_id map.
    pub(crate) async fn fetch(
        client: &Client,
        usernames: Vec<&str>,
        auth_token: &str,
    ) -> Result<Option<HashMap<String, String>>> {
        let endpoint = Self::endpoint(usernames)?;
        Self::send_request(client, endpoint, auth_token).await
    }

    fn endpoint(usernames: Vec<&str>) -> Result<Url> {
        let base_url = Url::parse(API_ENDPOINT_BASE).unwrap();
        let usernames = usernames.join(",");
        let mut url = Url::options()
            .base_url(Some(&base_url))
            .parse("users/by")
            .with_context(|| "Failed to parse users look up endpoint")?;
        url.set_query(Some(format!("usernames={}", usernames).as_str()));

        Ok(url)
    }

    async fn send_request(
        client: &Client,
        endpoint: Url,
        auth_token: &str,
    ) -> Result<Option<HashMap<String, String>>> {
        let response = client
            .get(endpoint)
            .bearer_auth(auth_token)
            .send()
            .await
            .with_context(|| "Request failed to get users")?;
        if !response.status().is_success() {
            warn!(
                "request not successful, got response status: {}",
                response.status()
            );
            return Ok(None);
        }

        let users: Users = response
            .json()
            .await
            .with_context(|| "Failed to deserialize json response")?;
        if users.errors.is_some() {
            warn!(
                "Errors occurred when requesting users: {:#?}",
                users.errors.unwrap()
            );
        }
        if let Some(users) = users.data {
            let user_ids = users
                .into_iter()
                .map(|data| (data.username, data.id))
                .collect();
            Ok(Some(user_ids))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use log::debug;
    use reqwest::Client;
    use serde_json::Result;

    use super::Users;
    use crate::twitter::API_ENDPOINT_BASE;

    #[test]
    fn endpoint() {
        let usernames = vec!["john", "mick"];
        let endpoint = Users::endpoint(usernames).unwrap();
        assert_eq!(
            format!("{}users/by?usernames={}", API_ENDPOINT_BASE, "john,mick"),
            endpoint.as_str()
        );
    }

    #[test]
    fn parse_users() -> Result<()> {
        let users_data = r#"
          {
            "data": [
              {
                "id": "2244994945",
                "name": "Twitter Dev",
                "username": "TwitterDev"
              }
            ],
            "errors": [
              {
                "value": "xn47mzh437",
                "detail": "Could not find user with usernames: [xn47mzh437].",
                "title": "Not Found Error",
                "resource_type": "user",
                "parameter": "usernames",
                "resource_id": "xn47mzh437",
                "type": "https://api.twitter.com/2/problems/resource-not-found"
              }
            ]
          }
        "#;

        let users: Users = serde_json::from_str(users_data)?;
        assert_eq!("2244994945", users.data.unwrap()[0].id);
        Ok(())
    }

    // To test this function:
    // RUST_LOG=debug cargo test fetch -- --ignored '[auth_token]' TwitterDev,jack,1ws23x
    #[tokio::test]
    #[ignore = "require command line input"]
    async fn fetch() {
        init();

        let mut args = std::env::args().rev();
        let arg = args.next().unwrap();
        let usernames = arg.split(',').collect();
        let auth_token = args.next().unwrap();

        let client = Client::new();
        let users = Users::fetch(&client, usernames, auth_token.as_str())
            .await
            .unwrap();
        if let Some(users) = users {
            debug!("Users: {:#?}", users);
        }
    }

    fn init() {
        let _ = env_logger::builder().try_init();
    }
}
