//! HTTP request.

use std::fmt::Debug;
use std::marker::Unpin;
use std::ops::Deref;
use std::sync::Arc;

use regex::Regex;
use serde::Deserialize;
use serde_json::{Deserializer, Value};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::{Cookies, Error, Head, Params, ToParameter};

/// HTTP request.
///
/// The request is fully loaded into memory. It's safe to clone
/// since the contents are behind an [`std::sync::Arc`].
#[derive(Debug, Clone, Default)]
pub struct Request {
    inner: Arc<Inner>,
    params: Option<Arc<Params>>,
}

#[derive(Debug, Default)]
struct Inner {
    head: Head,
    body: Vec<u8>,
    cookies: Cookies,
}

impl Request {
    /// Read the request in its entirety from a stream.
    pub async fn read(mut stream: impl AsyncRead + Unpin) -> Result<Self, Error> {
        let head = Head::read(&mut stream).await?;
        let content_length = head.content_length().unwrap_or(0);
        let mut body = vec![0u8; content_length];
        stream
            .read_exact(&mut body)
            .await
            .map_err(|_| Error::MalformedRequest("incorrect content length"))?;

        Ok(Request {
            params: None,
            inner: Arc::new(Inner {
                body,
                cookies: head.cookies(),
                head,
            }),
        })
    }

    pub fn with_params(mut self, params: Arc<Params>) -> Self {
        self.params = Some(params);
        self
    }

    /// Extract a parameter from the provided path.
    pub fn parameter<T: ToParameter>(&self, index: usize) -> Result<Option<T>, Error> {
        if let Some(ref params) = self.params {
            if let Some(parameter) = params.parameter(self.path().base(), index) {
                return Ok(Some(T::to_parameter(parameter)?));
            }
        }

        Ok(None)
    }

    /// Request's body as bytes.
    ///
    /// It's the job of the caller to handle encoding if any.
    pub fn body(&self) -> &[u8] {
        &self.inner.body
    }

    /// Request's body as JSON value.
    pub fn json_raw(&self) -> Result<Value, serde_json::Error> {
        self.json()
    }

    /// Request's body as HTML.
    /// UTF-8 encoding is assumed, and all incompatible characters are dropped.
    pub fn html(&self) -> String {
        String::from_utf8_lossy(self.body()).to_string()
    }

    /// Request's body deserialized from JSON into a particular Rust type.
    pub fn json<'a, T: Deserialize<'a>>(&'a self) -> Result<T, serde_json::Error> {
        let mut deserializer = Deserializer::from_slice(self.body());
        T::deserialize(&mut deserializer)
    }

    /// Request's cookies.
    pub fn cookies(&self) -> &Cookies {
        &self.inner.cookies
    }
}

impl Deref for Request {
    type Target = Head;

    fn deref(&self) -> &Self::Target {
        &self.inner.head
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_response() {
        #[derive(Deserialize)]
        struct Hello {
            hello: String,
        }

        let body = ("GET / HTTP/1.1\r\n".to_owned()
            + "Content-Type: application/json\r\n"
            + "Accept: */*\r\n"
            + "Content-Length: 18\r\n"
            + "\r\n"
            + r#"{"hello": "world"}"#)
            .as_bytes()
            .to_vec();
        let response = Request::read(&body[..]).await.expect("response");
        let json = response.json::<Hello>().expect("deserialize body");
        assert_eq!(json.hello, "world");
    }
}
