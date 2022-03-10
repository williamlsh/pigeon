use reqwest::{
    blocking::{Client, ClientBuilder, Response},
    header, Result,
};
use url::Url;

/// A reusable and blocking reqwest client.
#[derive(Debug)]
pub struct ReusableBlockingClient(Client);

impl ReusableBlockingClient {
    pub fn new(token: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(format!("Bearer {}", token).as_str())
                .expect("invalid header value"),
        );

        ReusableBlockingClient(
            ClientBuilder::new()
                .default_headers(headers)
                .build()
                .expect("could not build client with default headers"),
        )
    }

    pub fn get(&self, url: &Url) -> Result<Response> {
        self.0.get(url.as_str()).send()
    }
}

#[cfg(test)]
mod tests {
    use super::ReusableBlockingClient;
    use url::Url;

    #[test]
    fn test_get() {
        let http_client = ReusableBlockingClient::new("xxx");
        assert!(http_client
            .get(&Url::parse("https://httpbin.org/get").unwrap())
            .is_ok());
        assert!(http_client
            .get(&Url::parse("https://httpbin.org/headers").unwrap())
            .is_ok());
        assert!(http_client
            .get(&Url::parse("https://httpbin.org/ip").unwrap())
            .is_ok());
    }
}
