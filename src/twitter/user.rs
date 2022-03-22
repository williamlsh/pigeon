use log::warn;
use reqwest::blocking::Client;
use serde::Deserialize;
use url::Url;

const USER_LOOKUP_ENDPOINT_PATH: &str = "users/by";

#[derive(Debug, Deserialize)]
pub struct User {
    pub data: Vec<Data>,
}

impl User {
    /// Parameter usernames is a string contains comma separated usernames.
    pub fn url_from_usernames_query(base_url: &Url, usernames: &str) -> Result<Url, String> {
        let mut url = Url::options()
            .base_url(Some(base_url))
            .parse(USER_LOOKUP_ENDPOINT_PATH)
            .map_err(|error| format!("could not parse user lookup endpoint: {:?}", error))?;
        url.set_query(Some(format!("usernames={}", usernames).as_str()));

        Ok(url)
    }

    // No unit test for this function.
    /// get_user_ids returns user_ids corresponding to the orders of usernames.
    pub fn get_user_ids(
        client: &Client,
        endpoint: Url,
        auth_token: &str,
    ) -> Result<Option<Vec<String>>, String> {
        let response = client
            .get(endpoint)
            .bearer_auth(auth_token)
            .send()
            .map_err(|error| format!("get request failed: {:?}", error))?;
        if !response.status().is_success() {
            warn!(
                "request not successful, got response status: {}",
                response.status()
            );
            return Ok(None);
        }

        let user: User = response
            .json()
            .map_err(|error| format!("could not deserialize json response: {:?}", error))?;
        let user_ids: Vec<String> = user.data.into_iter().map(|data| data.id).collect();

        Ok(Some(user_ids))
    }
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[cfg(test)]
mod tests {
    use super::User;
    use crate::twitter::API_ENDPOINT_BASE;
    use serde_json::Result;
    use url::Url;

    #[test]
    fn test_url_from_usernames_query() {
        const USER_NAMES: &str = "john,mick";
        let base_url = Url::parse(API_ENDPOINT_BASE).unwrap();
        let endpoint = User::url_from_usernames_query(&base_url, USER_NAMES).unwrap();

        assert_eq!(
            format!("{}users/by?usernames={}", API_ENDPOINT_BASE, USER_NAMES),
            endpoint.as_str()
        );
    }

    #[test]
    fn test_parse_user() -> Result<()> {
        let user_data = r#"
        {
            "data": [
              {
                "id": "2244994945",
                "name": "Twitter Dev",
                "username": "TwitterDev"
              }
            ]
          }
        "#;

        let u: User = serde_json::from_str(user_data)?;

        assert_eq!("2244994945", u.data[0].id);

        Ok(())
    }
}
