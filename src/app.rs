use std::{
    env::current_exe,
    io::Read,
    path::PathBuf,
    time::Duration,
};

use crate::{
    types::version::{GameVersion, VersionMetadata},
    utils::net::{download_jre, get_version_metadata},
};

use anyhow::{anyhow, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{fs, task::JoinSet};

// ideally there is one public function for each subcommand

pub(crate) async fn install_versions(versions: Vec<&GameVersion>) -> Result<()> {
    let mut install_threads = JoinSet::new();
    let mut jre_threads = JoinSet::new();
    let bars = MultiProgress::new();

    let bar_style = ProgressStyle::with_template(
        "{prefix:.bold.blue.bright} {spinner:.green.bright} {wide_msg}",
    )
    .unwrap()
    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏-");

    let mut jres_installed: Vec<u8> = Vec::new();

    for version in versions {
        let pb_server = bars.add(ProgressBar::new_spinner());
        pb_server.set_style(bar_style.clone());
        pb_server.enable_steady_tick(Duration::from_millis(100));
        pb_server.set_prefix(format!("{}", version.id));

        pb_server.set_message("Getting version metadata...");
        let version_meta: VersionMetadata = get_version_metadata(version).await?;
        let jre_version = version_meta.java_version.major_version.clone();

        // spawn a thread to install the version
        install_threads.spawn(async move {
            if !version_meta.downloads.contains_key("server") {
                pb_server.finish_with_message("Cancelled (no server jar)");
                return Ok::<(), anyhow::Error>(());
            }

            let dir: PathBuf = current_exe()
                .unwrap_or_else(|e| panic!("Failed to get current executable path: {}", e))
                .parent()
                .unwrap_or_else(|| unreachable!())
                .join(".versions")
                .join(&version_meta.id.to_string());

            if dir.exists() {
                pb_server.finish_with_message("Cancelled (already installed)");
                return Ok::<(), anyhow::Error>(());
            }

            let url = version_meta
                .downloads
                .get("server")
                .unwrap_or_else(|| unreachable!())
                .url
                .clone();

            pb_server.set_message("Downloading server jar...");
            let server_jar = reqwest::get(url).await?.bytes().await?;

            // write to disk
            pb_server.set_message("Writing server jar to disk...");
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

            pb_server.finish_with_message("Done!");
            Ok::<(), anyhow::Error>(())
        });

        // if the JRE is already installed, skip it
        if jres_installed.contains(&jre_version) {
            continue;
        } else {
            jres_installed.push(jre_version);
        }

        let pb_jre = bars.add(ProgressBar::new_spinner());
        pb_jre.set_style(bar_style.clone());
        pb_jre.enable_steady_tick(Duration::from_millis(100));
        pb_jre.set_prefix(format!("JRE {} for {}", jre_version, version.id));

        // at the same time, spawn a thread to install the JRE
        jre_threads.spawn(async move {
            pb_jre.set_message("Installing JRE...");
            install_jre(&jre_version, &pb_jre).await?;
            pb_jre.finish_with_message("Done!");
            Ok::<(), anyhow::Error>(())
        });
    }

    while let Some(result) = install_threads.join_next().await {
        if let Err(e) = result? {
            let context = format!("Failed to install version: {}", e);
            return Err(anyhow!(context));
        }
    }

    while let Some(result) = jre_threads.join_next().await {
        if let Err(e) = result? {
            let context = format!("Failed to install JRE: {}", e);
            return Err(anyhow!(context));
        }
    }

    Ok(())
}

// pub(crate) async fn install_version(version: &GameVersion) -> Result<()> {
//     install_versions(vec![version]).await
// }

// major_version is 8, 11, 17 ONLY
async fn install_jre(major_version: &u8, pb: &ProgressBar) -> Result<()> {
    let jre_dir: PathBuf = current_exe()
        .unwrap_or_else(|e| panic!("Failed to get current executable path: {}", e))
        .parent()
        .unwrap_or_else(|| unreachable!())
        .join(".jre")
        .join(major_version.to_string());

    if jre_dir.exists() {
        pb.finish_with_message("Cancelled (already installed)");
        return Ok(());
    }

    pb.set_message("Downloading JRE...");
    let jre = download_jre(major_version).await?;

    pb.set_message("Extracting JRE...");

    {
        #![cfg(target_os = "linux")]

        use std::os::unix::fs::PermissionsExt;

        use bytes::Buf;
        use flate2::read::GzDecoder;
        use tar::Archive;

        // archive structure is jre*/bin/java
        // we want to extract the contents of jre* to the jre directory
        let mut reader = jre.reader();
        let tar = GzDecoder::new(&mut reader);
        let mut archive = Archive::new(tar);

        let mut entries = archive.entries()?;

        fs::create_dir_all(&jre_dir).await.unwrap_or_else(|e| {
            panic!(
                "Failed to create directory for JRE {}: {}",
                major_version, e
            )
        });

        while let Some(entry) = entries.next() {
            let mut entry = entry?;
            let path = entry.path()?;
            // strip the first directory
            let path: PathBuf = path.components().skip(1).collect();
            let path = jre_dir.join(path);
            entry.unpack(path)?;
        }

        // make the java binary executable
        let java_path = jre_dir.join("bin").join("java");
        let mut perms = fs::metadata(&java_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms).await?;

        // sanity check
        let java_path = jre_dir.join("bin").join("java");
        if !java_path.exists() {
            return Err(anyhow!("Failed to extract JRE"));
        }
    }

    {
        #![cfg(target_os = "windows")]

        use std::io::{Cursor, BufReader};

        use zip::ZipArchive;

        // same as above but with zip
        fs::create_dir_all(&jre_dir).await.unwrap_or_else(|e| {
            panic!(
                "Failed to create directory for JRE {}: {}",
                major_version, e
            )
        });

        let reader: BufReader<Cursor<Vec<u8>>> = BufReader::new(Cursor::new(jre.into()));
        let mut archive = ZipArchive::new(reader)?;

        // this crate is so bad
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let path = entry.enclosed_name().unwrap();

            // strip the first directory
            let path: PathBuf = path.components().skip(1).collect();
            let path = jre_dir.join(path);

            if entry.is_dir() {
                std::fs::create_dir_all(path)?;
                continue;
            }

            let mut buf = vec![0u8; entry.size() as usize];
            entry.read_exact(&mut buf)?;

            std::fs::write(path, buf)?; // async write breaks because ZipFile is not Send
        }

        // sanity check
        let java_path = jre_dir.join("bin").join("java.exe");
        if !java_path.exists() {
            return Err(anyhow!("Failed to extract JRE"));
        }
    }

    pb.finish_with_message("Done!");

    Ok(())
}
