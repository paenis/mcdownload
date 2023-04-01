use std::{env::current_exe, path::PathBuf, time::Duration};

use crate::{
    types::version::{GameVersion, VersionMetadata},
    utils::net::get_version_metadata,
};

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{fs, task::JoinSet};

pub(crate) async fn install_versions(versions: Vec<&GameVersion>) -> Result<()> {
    let mut install_threads = JoinSet::new();
    let bars = MultiProgress::new();

    for version in versions {
        let bar = bars.add(ProgressBar::new_spinner());
        bar.set_style(
            ProgressStyle::with_template(
                "{prefix:.bold.blue.bright} {spinner:.green.bright} {wide_msg}",
            )
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        bar.set_prefix(format!("{}", version.id));

        bar.set_message("Getting version metadata...");
        let version_meta: VersionMetadata = get_version_metadata(version).await?;

        install_threads.spawn(async move {
            if !version_meta.downloads.contains_key("server") {
                bar.finish_with_message("Cancelled (no server jar)");
                return Ok::<(), anyhow::Error>(());
            }

            let dir: PathBuf = current_exe()
                .unwrap_or_else(|e| panic!("Failed to get current executable path: {}", e))
                .parent()
                .unwrap_or_else(|| unreachable!())
                .join(".versions")
                .join(&version_meta.id.to_string());

            if dir.exists() {
                bar.finish_with_message("Cancelled (already installed)");
                return Ok::<(), anyhow::Error>(());
            }

            let url = version_meta
                .downloads
                .get("server")
                .unwrap_or_else(|| unreachable!())
                .url
                .clone();

            bar.set_message("Downloading server jar...");
            let server_jar = reqwest::get(url).await?.bytes().await?;

            // write to disk
            bar.set_message("Writing server jar to disk...");
            fs::create_dir_all(&dir).await.unwrap_or_else(|e| {
                panic!(
                    "Failed to create directory for version {}: {}",
                    version_meta.id, e
                )
            });
            fs::write(dir.join("server.jar"), server_jar)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to write server jar for version {}: {}",
                        version_meta.id, e
                    )
                });

            bar.finish_with_message("Done!");
            Ok::<(), anyhow::Error>(())
        });
    }

    while let Some(result) = install_threads.join_next().await {
        if let Err(e) = result? {
            let context = format!("Failed to install version: {}", e);
            return Err(e.context(context));
        }
    }

    Ok(())
}
