use anyhow::anyhow;
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Deserialize)]
pub struct RecaptchaResponse {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<HashSet<String>>,
}

pub async fn verify(
    key: &str,
    response: &str,
    user_ip: Option<&IpAddr>,
) -> Result<(), anyhow::Error> {
    let user_ip = user_ip.map(ToString::to_string);

    let mut url = Url::parse("https://www.google.com/recaptcha/api/siteverify").unwrap();

    // TODO: find a more secure means to pass the secret (i.e. headers)
    url.query_pairs_mut()
        .extend_pairs(&[("secret", key), ("response", response)]);

    if let Some(user_ip) = user_ip {
        url.query_pairs_mut().append_pair("remoteip", &user_ip);
    }

    let response = reqwest::get(url).await?;
    let recaptcha_response = response.json::<RecaptchaResponse>().await?;

    match (recaptcha_response.success, recaptcha_response.error_codes) {
        (true, _) => Ok(()),
        (false, Some(errors)) => Err(anyhow!(format!("{errors:?}"))),
        (false, _) => Err(anyhow!(format!("unknown error"))),
    }
}
