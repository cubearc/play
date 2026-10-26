use crate::protocol::FRAME_LIMIT;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use std::{io, time::Duration};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

pub type Channel = Framed<UnixStream, LinesCodec>;
pub fn channel(socket: UnixStream) -> Channel {
    Framed::new(socket, LinesCodec::new_with_max_length(FRAME_LIMIT))
}
pub async fn send<T: Serialize>(channel: &mut Channel, value: &T) -> io::Result<()> {
    let text =
        serde_json::to_string(value).map_err(|_| io::Error::other("Invalid terminal frame"))?;
    if text.len() > FRAME_LIMIT {
        return Err(io::Error::other("Terminal frame limit exceeded"));
    }
    tokio::time::timeout(Duration::from_secs(2), channel.send(text))
        .await
        .map_err(|_| io::Error::other("Terminal consumer is too slow"))?
        .map_err(|_| io::Error::other("Terminal transport closed"))
}
pub async fn receive<T: DeserializeOwned>(channel: &mut Channel) -> io::Result<Option<T>> {
    channel
        .next()
        .await
        .map(|line| {
            let text = line.map_err(|_| io::Error::other("Invalid terminal frame"))?;
            serde_json::from_str(&text).map_err(|_| io::Error::other("Invalid terminal frame"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Client, Server};
    #[tokio::test]
    async fn framing_preserves_partial_messages_and_rejects_oversized_or_invalid_data() {
        let (left, right) = UnixStream::pair().unwrap();
        let mut left = channel(left);
        let mut right = channel(right);
        send(&mut left, &Client::Probe).await.unwrap();
        assert!(matches!(
            receive::<Client>(&mut right).await.unwrap(),
            Some(Client::Probe)
        ));
        assert!(
            send(
                &mut left,
                &Client::Input {
                    data: "x".repeat(FRAME_LIMIT + 1)
                }
            )
            .await
            .is_err()
        );
        left.send("invalid json").await.unwrap();
        assert!(receive::<Server>(&mut right).await.is_err());
        drop(left);
        assert!(receive::<Server>(&mut right).await.unwrap().is_none());
    }
}
