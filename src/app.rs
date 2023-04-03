use std::{
    env::current_exe,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    types::{
        meta::InstanceSettings,
        version::{GameVersion, VersionMetadata},
    },
    utils::net::{download_jre, get_version_metadata},
};

use color_eyre::eyre::{self, eyre, Result, WrapErr};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{fs, task::JoinSet};

const DEFAULT_JVM_ARGS: &[&str] = &["-Xms4G", "-Xmx4G"];

// ideally there is one public function for each subcommand

pub(crate) async fn install_versions(versions: Vec<&GameVersion>) -> Result<()> {
    let mut install_threads = JoinSet::new();
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
                .join(&version_meta.id.to_string());

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
            let settings =
                InstanceSettings::new(DEFAULT_JVM_ARGS.iter().map(|s| s.to_string()).collect());

            write_settings(&settings, &instance_dir.join("settings.toml")).await?;

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
        pb_jre.set_style(bar_style.clone());
        pb_jre.enable_steady_tick(Duration::from_millis(100));
        pb_jre.set_prefix(format!("JRE {} for {}", jre_version, version.id));

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
    let jre_dir: PathBuf = current_exe()
        .wrap_err("Failed to get current executable path")?
        .parent()
        .expect("infallible")
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

async fn write_settings(settings: &InstanceSettings, path: &Path) -> Result<()> {
    fs::create_dir_all(path.parent().expect("infallible")).await?;
    let mut file = fs::File::create(path).await?;
    file.write_all(toml::to_string(settings)?.as_bytes())
        .await?;

    Ok(())
}

async fn read_settings(path: &Path) -> Result<InstanceSettings> {
    let mut file = fs::File::open(path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(toml::from_str(&contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_install_jre() {
        // remove the jre directory if the test panics
        std::panic::set_hook(Box::new(|_| {
            std::fs::remove_dir_all(
                current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(PathBuf::from(".jre/8")),
            )
            .unwrap();
        }));

        install_jre(&8, &ProgressBar::hidden()).await.unwrap();

        let path = current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(PathBuf::from(".jre/8/bin"));

        match std::env::consts::OS {
            "windows" => assert!(path.join("java.exe").exists()),
            "linux" => assert!(path.join("java").exists()),
            _ => assert!(false),
        }

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
