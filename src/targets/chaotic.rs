use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::chaotic::ChaoticTarget;
use reqwest;
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use std::time::Duration;    
use tokio::runtime::Runtime;
use url::Url;

impl LogFormatter for ChaoticTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}$repo/{}", mirror.url, arch)
    }
}

impl FetchMirrors for ChaoticTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://raw.githubusercontent.com/chaotic-aur/pkgbuild-chaotic-mirrorlist/main/mirrorlist";

        let output = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .text_with_charset("utf-8")
                    .await?,
            )
        })?;

        let urls = output
            .lines()
            .filter_map(|line| {
                if line.starts_with("# Server = ") {
                    Some(line.replace("# Server = ", ""))
                } else if line.starts_with("Server = ") {
                    Some(line.replace("Server = ", ""))
                } else {
                    None
                }
            })
            .filter_map(|line| Url::parse(&line.replace("$repo/$arch", "")).ok())
            .filter(|url| config.is_protocol_allowed_for_url(url));
        let result: Vec<_> = urls
            .map(|url| {
                let url_to_test = url
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");
                Mirror {
                    country: None,
                    url,
                    url_to_test,
                }
            })
            .collect();

        Ok(result)
    }
}
