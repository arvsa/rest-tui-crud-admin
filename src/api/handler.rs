use reqwest::Client;
use serde::Serialize;
use std::error::Error;
use tokio::time::{timeout, Duration};

pub struct ApiServiceHandler {
    client: Client,
}

impl Default for ApiServiceHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiServiceHandler {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let request = async {
            let mut req = self.client.get(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            let res = req.send().await?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                return Err(format!("[API ERROR] GET {} — {}", status, body).into());
            }
            let data = res.json::<serde_json::Value>().await?;
            Ok(data)
        };

        match timeout(Duration::from_secs(10), request).await {
            Ok(res) => res,
            Err(_) => Err("[API ERROR] timeout".into()),
        }
    }

    pub async fn post_json<T>(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &T,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        T: Serialize + ?Sized,
    {
        let request = async {
            let mut req = self.client.post(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            let res = req.json(body).send().await?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                return Err(format!("[API ERROR] POST {} — {}", status, body).into());
            }
            Ok(())
        };

        match timeout(Duration::from_secs(10), request).await {
            Ok(res) => res,
            Err(_) => Err("[API ERROR] timeout".into()),
        }
    }

    pub async fn put_json<T>(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &T,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        T: Serialize + ?Sized,
    {
        let request = async {
            let mut req = self.client.put(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            let res = req.json(body).send().await?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                return Err(format!("[API ERROR] PUT {} — {}", status, body).into());
            }
            Ok(())
        };

        match timeout(Duration::from_secs(10), request).await {
            Ok(res) => res,
            Err(_) => Err("[API ERROR] timeout".into()),
        }
    }

    pub async fn delete(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let request = async {
            let mut req = self.client.delete(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            let res = req.send().await?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                return Err(format!("[API ERROR] DELETE {} — {}", status, body).into());
            }
            Ok(())
        };

        match timeout(Duration::from_secs(10), request).await {
            Ok(res) => res,
            Err(_) => Err("[API ERROR] timeout".into()),
        }
    }
}
