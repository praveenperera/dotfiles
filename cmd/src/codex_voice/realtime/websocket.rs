use color_eyre::eyre::{Result, WrapErr};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::Request, Message},
    MaybeTlsStream, WebSocketStream,
};

pub struct RealtimeWebSocket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RealtimeWebSocket {
    pub async fn connect(api_key: &str, model: &str) -> Result<Self> {
        let request = realtime_request(api_key, model)?;
        let (stream, _) = connect_async(request)
            .await
            .wrap_err("Failed to connect to OpenAI Realtime")?;
        Ok(Self { stream })
    }

    pub async fn send_json(&mut self, event: impl Serialize) -> Result<()> {
        let text = serde_json::to_string(&event)?;
        self.stream
            .send(Message::Text(text.into()))
            .await
            .wrap_err("Failed to send Realtime event")
    }

    pub async fn next_text(&mut self) -> Result<Option<String>> {
        while let Some(message) = self.stream.next().await {
            let message = message.wrap_err("Failed to read Realtime event")?;
            match message {
                Message::Text(text) => return Ok(Some(text.to_string())),
                Message::Close(_) => return Ok(None),
                Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {}
            }
        }
        Ok(None)
    }
}

fn realtime_request(api_key: &str, model: &str) -> Result<Request<()>> {
    let url = format!("wss://api.openai.com/v1/realtime?model={model}");
    let mut request = url
        .into_client_request()
        .wrap_err("Failed to build OpenAI Realtime request")?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {api_key}")
            .parse()
            .wrap_err("Failed to build Authorization header")?,
    );
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::realtime_request;

    #[test]
    fn builds_ga_realtime_request_without_beta_header() {
        let request = realtime_request("key", "gpt-realtime-2").unwrap();

        assert_eq!(
            request.uri().to_string(),
            "wss://api.openai.com/v1/realtime?model=gpt-realtime-2"
        );
        assert_eq!(request.headers()["Authorization"], "Bearer key");
        assert!(!request.headers().contains_key("OpenAI-Beta"));
    }
}
