use reqwest::Client;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

pub struct Scrobbler {
    pub api_key: String,
    pub api_secret: String,
    pub session_key: Option<String>,
    client: Client,
}

impl Scrobbler {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            session_key: None,
            client: Client::new(),
        }
    }

    pub fn new_with_session(api_key: String, api_secret: String, session_key: String) -> Self {
        Self {
            api_key,
            api_secret,
            session_key: Some(session_key),
            client: Client::new(),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.session_key.is_some()
    }

    pub async fn get_token(&self) -> Option<String> {
        let mut params = HashMap::new();
        params.insert("method", "auth.getToken");
        params.insert("api_key", &self.api_key);

        let sig = self.sign(&params);
        let resp = match self
            .client
            .get(API_URL)
            .query(&[
                ("method", "auth.getToken"),
                ("api_key", &self.api_key),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(_) => return None,
        };

        let json: serde_json::Value = match resp.json().await {
            Ok(json) => json,
            Err(_) => return None,
        };

        json["token"].as_str().map(|token| token.to_string())
    }

    pub fn auth_url(&self, token: &str) -> String {
        format!(
            "https://www.last.fm/api/auth/?api_key={}&token={}",
            self.api_key, token
        )
    }

    pub async fn get_session(&mut self, token: &str) -> bool {
        let mut params = HashMap::new();
        params.insert("method", "auth.getSession");
        params.insert("api_key", &self.api_key);
        params.insert("token", token);

        let sig = self.sign(&params);
        let resp = self
            .client
            .get(API_URL)
            .query(&[
                ("method", "auth.getSession"),
                ("api_key", &self.api_key),
                ("token", token),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await;

        match resp {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    if let Some(session_key) = json["session"]["key"].as_str() {
                        self.session_key = Some(session_key.to_string());
                        return true;
                    }
                }
                Err(error) => eprintln!("Failed to parse response: {}", error),
            },
            Err(error) => eprintln!("Request failed: {}", error),
        }

        false
    }

    pub async fn update_now_playing(&self, artist: &str, track: &str, album: &str) {
        let Some(session_key) = &self.session_key else {
            return;
        };

        let mut params = HashMap::new();
        params.insert("method", "track.updateNowPlaying");
        params.insert("api_key", &self.api_key);
        params.insert("sk", session_key);
        params.insert("artist", artist);
        params.insert("track", track);
        params.insert("album", album);

        let sig = self.sign(&params);

        let _ = self
            .client
            .post(API_URL)
            .form(&[
                ("method", "track.updateNowPlaying"),
                ("api_key", &self.api_key),
                ("sk", session_key),
                ("artist", artist),
                ("track", track),
                ("album", album),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await;
    }

    pub async fn scrobble(&self, artist: &str, track: &str, album: &str) {
        let Some(session_key) = &self.session_key else {
            return;
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let mut params = HashMap::new();
        params.insert("method", "track.scrobble");
        params.insert("api_key", &self.api_key);
        params.insert("sk", session_key);
        params.insert("artist", artist);
        params.insert("track", track);
        params.insert("album", album);
        params.insert("timestamp", &timestamp);

        let sig = self.sign(&params);

        let _ = self
            .client
            .post(API_URL)
            .form(&[
                ("method", "track.scrobble"),
                ("api_key", &self.api_key),
                ("sk", session_key),
                ("artist", artist),
                ("track", track),
                ("album", album),
                ("timestamp", &timestamp),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await;
    }

    fn sign(&self, params: &HashMap<&str, &str>) -> String {
        let mut keys: Vec<&&str> = params.keys().collect();
        keys.sort();

        let mut base = String::new();
        for key in keys {
            base.push_str(key);
            base.push_str(params[key]);
        }
        base.push_str(&self.api_secret);

        format!("{:x}", md5::compute(base.as_bytes()))
    }
}
