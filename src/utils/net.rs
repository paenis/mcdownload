const PISTON_API_URL: &str = "https://piston-meta.mojang.com/";
const FABRIC_API_URL: &str = "https://meta.fabricmc.net/";

pub(crate) fn api_path(path: &str) -> String {
    format!("{}{}", PISTON_API_URL, path)
}

pub(crate) fn fabric_api_path(path: &str) -> String {
    format!("{}{}", FABRIC_API_URL, path)
}