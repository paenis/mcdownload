use std::env::current_exe;
use std::path::PathBuf;
use std::time::Duration;

use color_eyre::eyre::{self, eyre, Result, WrapErr};
use dialoguer::Confirm;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use tokio::fs;
use tokio::process::Command;
use tokio::task::JoinSet;

use crate::types::meta::InstanceSettings;
use crate::types::version::{GameVersion, VersionMetadata, VersionNumber};
use crate::utils::net::{download_jre, get_version_metadata};

lazy_static! {
    static ref CURRENT_DIR: PathBuf = current_exe()
        .unwrap()
        .parent()
        .expect("infallible")
        .to_path_buf();
    static ref INSTANCE_BASE_DIR: PathBuf = CURRENT_DIR.join(".versions");
    static ref JRE_BASE_DIR: PathBuf = CURRENT_DIR.join(".jre");
    static ref PB_STYLE: ProgressStyle = ProgressStyle::with_template(
        "{prefix:.bold.blue.bright} {spinner:.green.bright} {wide_msg}",
    )
    .unwrap()
    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏-");
}

// ideally there is one public function for each subcommand

pub(crate) async fn install_versions(versions: Vec<&GameVersion>) -> Result<()> {
    let mut install_threads = JoinSet::new();
    let bars = MultiProgress::new();

    let mut jres_installed: Vec<u8> = Vec::new();

    for version in versions {
        let pb_server = bars.add(ProgressBar::new_spinner());
        pb_server.set_style(PB_STYLE.clone());
        pb_server.set_prefix(format!("{}", version.id));
        pb_server.enable_steady_tick(Duration::from_millis(100));

        pb_server.set_message("Getting version metadata...");
        let version_meta: VersionMetadata = get_version_metadata(version).await?;
        let jre_version = version_meta.java_version.major_version;

        // spawn a thread to install the version
        install_threads.spawn(async move {
            if !version_meta.downloads.contains_key("server") {
                pb_server.finish_with_message("Cancelled (no server jar)");
                return Ok::<(), eyre::Report>(());
            }

            let instance_dir: PathBuf = current_exe()
                .wrap_err("Failed to get current executable path")?
                .parent()
                .expect("infallible")
                .join(".versions")
                .join(version_meta.id.to_string());

            if instance_dir.exists() {
                pb_server.finish_with_message("Cancelled (already installed)");
                return Ok::<(), eyre::Report>(());
            }

            let url = version_meta
                .downloads
                .get("server")
                .expect("infallible")
                .url
                .clone();

            pb_server.set_message("Downloading server jar...");
            let server_jar = reqwest::get(url).await?.bytes().await?;

            // write to disk
            pb_server.set_message("Writing server jar to disk...");
            fs::create_dir_all(&instance_dir).await.wrap_err(format!(
                "Failed to create instance directory for {}",
                version_meta.id
            ))?;

            fs::write(instance_dir.join("server.jar"), server_jar)
                .await
                .wrap_err(format!(
                    "Failed to write server jar for {}",
                    version_meta.id
                ))?;

            // write settings
            pb_server.set_message("Writing settings...");
            let settings = InstanceSettings::new(jre_version);

            settings.save(&instance_dir.join("settings.toml")).await?;

            pb_server.finish_with_message("Done!");
            Ok::<(), eyre::Report>(())
        });

        // if the JRE is already installed, skip it
        if jres_installed.contains(&jre_version) {
            continue;
        } else {
            jres_installed.push(jre_version);
        }

        let pb_jre = bars.add(ProgressBar::new_spinner());
        pb_jre.set_style(PB_STYLE.clone());
        pb_jre.set_prefix(format!("JRE {} for {}", jre_version, version.id));
        pb_jre.enable_steady_tick(Duration::from_millis(100));

        // at the same time, spawn a thread to install the JRE
        install_threads.spawn(async move {
            pb_jre.set_message("Installing JRE...");
            install_jre(&jre_version, &pb_jre)
                .await
                .wrap_err(format!("Failed to install JRE {jre_version}"))?;

            Ok::<(), eyre::Report>(())
        });
    }

    while let Some(result) = install_threads.join_next().await {
        result?.wrap_err("Failed to install server or JRE")?;
    }

    Ok(())
}

// pub(crate) async fn install_version(version: &GameVersion) -> Result<()> {
//     install_versions(vec![version]).await
// }

// major_version is 8, 16, 17 ONLY
async fn install_jre(major_version: &u8, pb: &ProgressBar) -> Result<()> {
    let jre_dir = JRE_BASE_DIR.join(major_version.to_string());

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

        std::fs::create_dir_all(&jre_dir).wrap_err(format!(
            "Failed to create directory for JRE {major_version}"
        ))?;

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
        let mut perms = std::fs::metadata(&java_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&java_path, perms)?;

        // sanity check
        let java_path = jre_dir.join("bin").join("java");
        if !java_path.exists() {
            return Err(eyre!(
                "Failed to extract JRE ({} does not exist)",
                java_path.display()
            ));
        }
    }

    {
        #![cfg(target_os = "windows")]

        use std::io::{BufReader, Cursor, Read};

        use zip::ZipArchive;

        // same as above but with zip
        fs::create_dir_all(&jre_dir).await.wrap_err(format!(
            "Failed to create directory for JRE {major_version}",
        ))?;

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
            return Err(eyre!(
                "Failed to extract JRE ({} does not exist)",
                java_path.display()
            ));
        }
    }

    pb.finish_with_message("Done!");

    Ok(())
}

pub(crate) async fn run_version(id: VersionNumber) -> Result<()> {
    let instance_path = INSTANCE_BASE_DIR.join(id.to_string());

    if !instance_path.exists() {
        return Err(eyre!("Version {} is not installed", id));
    }

    let settings = InstanceSettings::from_file(instance_path.join("settings.toml")).await?;

    // check if the JRE is installed and install it if not
    let jre_version = settings.java.version;
    let jre_dir = JRE_BASE_DIR.join(jre_version.to_string());

    if !jre_dir.exists() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(PB_STYLE.clone());
        pb.set_prefix(format!("JRE {} for {}", jre_version, id));
        pb.enable_steady_tick(Duration::from_millis(100));

        install_jre(&jre_version, &pb).await?;
    }

    let java_path = get_java_path(jre_version);

    let mut cmd = Command::new(&java_path);

    cmd.current_dir(&instance_path);
    cmd.kill_on_drop(true);

    cmd.args(settings.java.args);
    cmd.arg("-jar");
    cmd.arg(settings.server.jar);
    cmd.args(settings.server.args);

    let mut child = cmd.spawn().wrap_err("Failed to start server")?;
    let status = child.wait().await.wrap_err("Failed to wait for server")?; // says it closes the stdin but it doesn't (i guess)

    if !status.success() {
        let upload = Confirm::new()
            .with_prompt("Server exited with an error. Would you like to upload the crash report?")
            .default(false)
            .interact()?;

        if upload {
            let crash_reports = instance_path.join("crash-reports");

            let latest = std::fs::read_dir(crash_reports)
                .wrap_err("Failed to read crash reports directory")?
                .filter_map(|entry| entry.ok())
                .max_by(|a, b| {
                    let a = a.metadata().unwrap().modified().unwrap();
                    let b = b.metadata().unwrap().modified().unwrap();

                    a.cmp(&b)
                })
                .ok_or_else(|| eyre!("No crash reports found"))?;

            let content =
                std::fs::read_to_string(latest.path()).wrap_err("Failed to read crash report")?;

            // upload to mclo.gs
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.mclo.gs/1/log")
                .form(&[("content", content)])
                .send()
                .await?;

            // parse json response
            let response: serde_json::Value = response.json().await?;

            if response["success"].as_bool().unwrap() {
                println!(
                    "Crash report uploaded to {}",
                    response["url"].as_str().unwrap()
                );
            } else {
                return Err(eyre!(
                    "Failed to upload crash report: {}",
                    response["error"].as_str().unwrap()
                ));
            }
        }

        return Err(eyre!("Server exited with {status}"));
    }

    Ok(())
}

fn get_java_path(version: u8) -> PathBuf {
    let mut path = JRE_BASE_DIR.join(version.to_string());
    path.push("bin");

    match std::env::consts::OS {
        "windows" => path.push("java.exe"),
        "linux" => path.push("java"),
        _ => panic!("Unsupported OS"),
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_install_jre() {
        // remove the jre directory if the test panics
        scopeguard::defer! {
            let path = JRE_BASE_DIR.join("8");

            if path.exists() {
                std::fs::remove_dir_all(path).unwrap();
            }
        }

        install_jre(&8, &ProgressBar::hidden()).await.unwrap();

        let path = JRE_BASE_DIR.join("8/bin");

        match std::env::consts::OS {
            "windows" => assert!(path.join("java.exe").exists()),
            "linux" => assert!(path.join("java").exists()),
            _ => assert!(false),
        }
    }
}
