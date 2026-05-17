use serde::Serialize;

use crate::credential::ResolvedTtsCredential;
use crate::error::XzTtsError;

pub async fn post_json<T>(
    url: &str,
    credential: &ResolvedTtsCredential,
    body: &T,
) -> Result<reqwest::Response, XzTtsError>
where
    T: Serialize + ?Sized,
{
    reqwest::Client::new()
        .post(url)
        .header("X-Api-App-Key", &credential.app_id)
        .header("X-Api-Access-Key", &credential.access_token)
        .header("X-Api-Resource-Id", &credential.resource_id)
        .json(body)
        .send()
        .await
        .map_err(|err| {
            if err.is_timeout() {
                XzTtsError::Timeout {
                    message: format!("http request timed out: {err}"),
                }
            } else {
                XzTtsError::Network {
                    message: format!("http request failed: {err}"),
                }
            }
        })
}

pub async fn download_bytes(url: &str) -> Result<Vec<u8>, XzTtsError> {
    let response = reqwest::Client::new().get(url).send().await.map_err(|err| {
        if err.is_timeout() {
            XzTtsError::Timeout {
                message: format!("http download timed out: {err}"),
            }
        } else {
            XzTtsError::Network {
                message: format!("http download failed: {err}"),
            }
        }
    })?;

    let status = response.status();
    if status.as_u16() == 403 || status.as_u16() == 404 {
        return Err(XzTtsError::Format {
            message: "audio URL expired, re-query needed".into(),
        });
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        return Err(XzTtsError::Network {
            message: format!("HTTP {}: {}", status.as_u16(), body),
        });
    }

    let bytes = response.bytes().await.map_err(|err| XzTtsError::Network {
        message: format!("failed to read download body: {err}"),
    })?;

    if bytes.is_empty() {
        return Err(XzTtsError::Format {
            message: "downloaded empty audio".into(),
        });
    }

    Ok(bytes.to_vec())
}
