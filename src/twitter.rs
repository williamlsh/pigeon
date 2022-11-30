mod timeline;
mod user;

pub(crate) use user::Users;
pub(crate) use timeline::Timeline;

const API_ENDPOINT_BASE: &str = "https://api.twitter.com/2/";
