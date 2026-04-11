




use anyhow::Result;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;


#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Bytes>,
}

impl Request {
    pub fn get(url: &str) -> Self {
        Request {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn post(url: &str, body: Bytes) -> Self {
        let mut req = Request::get(url);
        req.method = "POST".to_string();
        req.body = Some(body);
        req
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }
}


#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl Response {
    pub fn new(status: u16, body: Bytes) -> Self {
        Response {
            status,
            headers: HashMap::new(),
            body,
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}


pub struct HttpClient {
    user_agent: String,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            user_agent: "Atrium/0.1".to_string(),
        }
    }

    pub fn user_agent(mut self, agent: &str) -> Self {
        self.user_agent = agent.to_string();
        self
    }

    
    pub async fn send(&self, request: &Request) -> Result<Response> {
        
        
        
        let mut headers = request.headers.clone();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());

        
        Ok(Response {
            status: 200,
            headers,
            body: Bytes::from("<html><body>Mock Response</body></html>"),
        })
    }

    
    pub async fn get(&self, url: &str) -> Result<Response> {
        let request = Request::get(url);
        self.send(&request).await
    }

    
    pub async fn post(&self, url: &str, body: Bytes) -> Result<Response> {
        let request = Request::post(url, body);
        self.send(&request).await
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}


pub struct ResourceLoader {
    client: HttpClient,
    cache: HashMap<String, Response>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        ResourceLoader {
            client: HttpClient::new(),
            cache: HashMap::new(),
        }
    }

    
    pub async fn fetch(&mut self, url: &str) -> Result<Response> {
        
        if let Some(cached) = self.cache.get(url) {
            return Ok(cached.clone());
        }

        
        let response = self.client.get(url).await?;
        
        
        if response.ok() {
            self.cache.insert(url.to_string(), response.clone());
        }

        Ok(response)
    }

    
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_builder() {
        let request = Request::get("https://example.com")
            .header("Accept", "text/html")
            .header("Accept-Language", "en-US");

        assert_eq!(request.url, "https://example.com");
        assert_eq!(request.method, "GET");
        assert!(request.headers.contains_key("Accept"));
    }

    #[tokio::test]
    async fn test_response() {
        let response = Response::new(200, Bytes::from("Hello"));
        assert!(response.ok());
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn test_http_client() {
        let client = HttpClient::new().user_agent("TestAgent/1.0");
        let response = client.get("https://example.com").await.unwrap();

        assert!(response.ok());
    }
}
