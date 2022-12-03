mod timeline;
mod users;

pub(crate) use users::Users;
pub(crate) use timeline::{Timeline, UrlBuilder, PaginationToken, Data as Tweet};

const API_ENDPOINT_BASE: &str = "https://api.twitter.com/2/";
