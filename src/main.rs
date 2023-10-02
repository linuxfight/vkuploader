use reqwest::{Client};
use reqwest::multipart::{Form, Part};
use serde::{Serialize, Deserialize};
use tokio::fs::read;
use std::fs::{File, read_dir};
use std::io::{BufReader};
use std::path::PathBuf;
use progress_bar::*;
use num_integer::div_ceil;

#[derive(Serialize, Deserialize)]
struct Config {
    token: String,
    group_id: String,
    max_uploads: usize,
    v: String,
    albums: Vec<Album>
}

#[derive(Serialize, Deserialize)]
struct Album {
    album_id: String,
    path: PathBuf
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_file = File::open("config.json")?;
    let reader = BufReader::new(config_file);
    let config: Config = serde_json::from_reader(reader)?;
    let client = Client::new();

    for album in &config.albums {
        let mut file_number = 1;
        let mut form = Form::new();
        let entries: Vec<_> = read_dir(&album.path)?.collect();
        init_progress_bar(div_ceil(entries.len(), config.max_uploads));
        set_progress_bar_action(format!("Album {}", album.album_id).as_str(), Color::Blue, Style::Bold);
        for (_index, entry) in entries.into_iter().enumerate() {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                let file_data = read(&file_path).await?;
                let file_name = file_path.file_name().unwrap().to_str().unwrap();
                let part = Part::stream(reqwest::Body::from(file_data))
                    .file_name(file_name.to_string())
                    .mime_str("image/png")?;
                form = form.part(format!("file{}", file_number),part);
                file_number += 1;

                if file_number > config.max_uploads {
                    upload_photos(&client, &album, &config, form).await?;
                    inc_progress_bar();
                    file_number = 1;
                    form = Form::new();
                }
            }
        }

        if file_number > 1 {
            upload_photos(&client, &album, &config, form).await?;
            inc_progress_bar();
        }

        finalize_progress_bar();
    }

    Ok(())
}

async fn upload_photos(
    client: &Client,
    album: &Album,
    config: &Config,
    form: Form
) -> Result<(), Box<dyn std::error::Error>> {
    let resp = &client
        .post("https://api.vk.com/method/photos.getUploadServer")
        .form(&[
            ("album_id", &album.album_id),
            ("access_token", &config.token),
            ("group_id", &config.group_id),
            ("v", &config.v),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let upload_url = resp["response"]["upload_url"]
        .as_str()
        .ok_or("Unable to get upload server")?;

    let resp = &client
        .post(upload_url)
        .multipart(form)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let upload_server = resp["server"]
        .as_u64()
        .ok_or("Unable to get server")?;

    let photos_list = resp["photos_list"].as_str()
        .ok_or("Unable to get photos list")?;

    let upload_hash = resp["hash"].as_str()
        .ok_or("Unable to get upload hash")?;

    let _resp = &client
        .post("https://api.vk.com/method/photos.save")
        .form(&[
            ("album_id", &album.album_id.as_str()),
            ("group_id", &config.group_id.as_str()),
            ("access_token", &config.token.as_str()),
            ("v", &config.v.as_str()),
            ("hash", &upload_hash),
            ("server", &upload_server.to_string().as_str()),
            ("photos_list", &photos_list),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(())
}