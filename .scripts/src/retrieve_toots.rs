use regex::Regex;
use reqwest::header::HeaderValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::thread::sleep;
use std::time::Duration;

static RETRIEVE_NUM_TOOTS: usize = 1000;
static SERVER: &str = "fosstodon.org";
static MUID: &str = "108219415927856966";

fn reformat_toot(x: &mut serde_json::Value) -> Result<(), String> {
    let toot = x
        .as_object_mut()
        .ok_or_else(|| "JSON not an object".to_string())?;

    // URL -> Syndication
    toot.remove("uri");
    let toot_url = toot.remove("url")
        .ok_or_else(|| "Missing URL".to_string())?;
    toot.insert("syndication".to_string(), toot_url);

    // Created At -> Date
    let toot_date = toot.remove("created_at")
        .ok_or_else(|| "Missing created_at".to_string())?;
    toot.insert("date".to_string(), toot_date);

    // Strip out highly dynamic account information
    let account: &mut tera::Map<String, serde_json::Value> = toot
        .get_mut("account")
        .ok_or_else(|| "Missing account field".to_string())?
        .as_object_mut()
        .ok_or_else(|| "Account field not an object".to_string())?;

    // Doesn't matter if we fail to remove these
    account.remove("avatar_static");
    account.remove("header_static");
    account.remove("noindex");
    account.remove("roles");
    account.remove("locked");
    account.remove("bot");
    account.remove("discoverable");
    account.remove("group");
    account.remove("created_at");
    account.remove("note");
    account.remove("followers_count");
    account.remove("following_count");
    account.remove("statuses_count");
    account.remove("last_status_at");
    account.remove("emojis");
    account.remove("fields");

    Ok(())
}

fn parse_link_header(header: &HeaderValue) -> Result<HashMap<String, String>, String> {
    let mut links = HashMap::new();
    let re = Regex::new(r#"<([^>]*)>;\s*rel="([^"]*)""#)
        .map_err(|_| "Regex compilation failed".to_string())?;

    let header_str = header
        .to_str()
        .map_err(|v| v.to_string())?;

    for cap in re.captures_iter(header_str) {
        if let (Some(url), Some(rel)) = (cap.get(1), cap.get(2)) {
            links.insert(rel.as_str().to_owned(), url.as_str().to_owned());
        }
    }

    Ok(links)
}

// TODO: Make sure that the JSON blobs aren't
// the same!
fn write_json_to_file(item: &serde_json::Value, path: &str) -> Result<(), String> {
    let item_str = serde_json::to_string(&item)
        .map_err(|x| x.to_string())?;

    let mut file = File::create(path)
        .map_err(|x| x.to_string())?;

    // Write the content to the file
    file.write_all(item_str.as_bytes())
        .map_err(|x| x.to_string())?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();

    let mut url = format!(
        "https://{}/api/v1/accounts/{}/statuses?limit={}",
        SERVER, MUID, RETRIEVE_NUM_TOOTS
    );

    let mut more_toots_exist = true;

    while more_toots_exist {
        let response = client
            .get(url.clone())
            .send()
            .await
            .expect("Unable to reach server");

        // Note: Cannot clone entire response
        let headers = response.headers().clone();

        // Note: .text() consumes response
        let text_response = response
            .text()
            .await
            .expect("Unable to grab response text");

        let mut json_response: serde_json::Value =
            serde_json::from_str(&text_response).expect("JSON parse error");

        let json_array = json_response
            .as_array_mut()
            .expect("Expected JSON Array");

        for item in json_array.iter_mut() {
            reformat_toot(item).unwrap();

            let toot_id = item
                .get("id")
                .and_then(|x| x.as_str())
                .expect("Failed to get toot id");

            let data_dir = "../.data";
            let data_dir_exists = std::fs::metadata(data_dir)
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false);
            if !data_dir_exists {
                std::fs::create_dir(data_dir)
                    .expect("Failed to create directory");
            }

            let path: String = format!("{}/{}.json", data_dir, toot_id);

            let mut update_json = false;

            let f = File::open(path.clone());
            if f.is_ok() {
                let reader = BufReader::new(f.unwrap());
                let f_json_result: Result<serde_json::Value, _> = serde_json::from_reader(reader);
                if f_json_result.is_ok() {
                    // If the JSON from the network is different, update
                    if f_json_result.unwrap() != *item {
                        update_json = true;
                    }
                }
            } else {
                // File doesn't exist (probably) so
                // try to write the new JSON
                update_json = true;
            }

            if update_json {
                write_json_to_file(&item, path.as_ref())
                .expect("Failed to write to file");
            }


        }
        println!("Retrieved {} toots from server", json_array.len());

        let next_url_result: Result<String, String> = headers
            .get("link")
            .ok_or_else(|| "No header link".to_string())
            .and_then(parse_link_header)
            .and_then(|v| {
                v.get("next")
                    .cloned()
                    .ok_or_else(|| "No next tag".to_string())
            });

        match next_url_result {
            Ok(next_url) => url = next_url,
            Err(_) => more_toots_exist = false,
        }

        // Sleep for the 1 second to not overload the server
        sleep(Duration::new(1, 0));
    }
}
