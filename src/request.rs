use std::collections::HashMap;

use anyhow::anyhow;
use async_process::{Command, Stdio};
use bytes::Bytes;
use reqwest::{Body, Client, Request, RequestBuilder, multipart};
use tokio::{self, fs as tfs};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::Arg;

pub(crate) async fn build_request(
    arg: &Arg,
    client: &Client,
) -> anyhow::Result<Request> {
    let mut builder = client.request(
        arg.method.to_reqwest_method(),
        arg.url.as_ref().unwrap().clone(),
    );

    // the following four types are mutually exclusive
    // only one will take effect
    builder = set_request_text_body(arg, builder).await?;
    builder = set_request_form_body(arg, builder).await?;
    builder = set_request_json_body(arg, builder).await?;
    builder = set_request_multipart_body(arg, builder).await?;

    match builder.build() {
        Ok(client) => Ok(client),
        Err(e) => Err(Box::new(e).into()),
    }
}

async fn set_request_text_body(
    arg: &Arg,
    mut builder: RequestBuilder,
) -> anyhow::Result<RequestBuilder> {
    if let Some(text_body) = &arg.text_body {
        builder = builder
            .body(Bytes::from(text_body.clone()))
            .header("Content-Type", "text/plain; charset=UTF-8");
    }

    if let Some(text_file) = &arg.text_file {
        let file = tfs::File::open(text_file).await?;
        builder = builder
            .body(file)
            .header("Content-Type", "text/plain; charset=UTF-8");
    }

    Ok(builder)
}

async fn set_request_json_body(
    arg: &Arg,
    mut builder: RequestBuilder,
) -> anyhow::Result<RequestBuilder> {
    if let Some(json_body) = &arg.json_body {
        builder = builder
            .body(Bytes::from(json_body.clone()))
            .header("Content-Type", "application/json; charset=UTF-8");
    }

    if let Some(json_file) = &arg.json_file {
        let file = tfs::File::open(json_file).await?;
        builder = builder
            .body(file)
            .header("Content-Type", "application/json; charset=UTF-8");
    }

    if let Some(json_command) = &arg.json_command {
        let inputs: Vec<&str> = json_command.split(" ").collect();
        let inputs: Vec<&&str> =
            inputs.iter().filter(|&&x| !x.is_empty()).collect();

        if inputs.is_empty() {
            anyhow::bail!(anyhow!("invalid json command"))
        }

        let mut cmd = Command::new(inputs[0]);
        cmd.stdin(Stdio::null());
        for arg in &inputs[1..] {
            cmd.arg(arg);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!(anyhow!(format!(
                "external command execute failed, exit code: {}",
                output.status
            )))
        }
        builder = builder
            .body(output.stdout)
            .header("Content-Type", "application/json; charset=UTF-8");
    }

    Ok(builder)
}

async fn set_request_form_body(
    arg: &Arg,
    mut builder: RequestBuilder,
) -> anyhow::Result<RequestBuilder> {
    if !arg.form.is_empty() {
        let mut params = HashMap::new();
        for kv in &arg.form {
            let parts = kv.trim().split_once(':');
            if let Some(v) = parts {
                params.insert(v.0, v.1);
            }
        }
        builder = builder.form(&params);
    }

    Ok(builder)
}

async fn set_request_multipart_body(
    arg: &Arg,
    mut builder: RequestBuilder,
) -> anyhow::Result<RequestBuilder> {
    if !arg.mp.is_empty() || !arg.mp_file.is_empty() {
        let mut form = multipart::Form::new();
        for kv in &arg.mp {
            let parts = kv.trim().split_once(':');
            if let Some(parts) = parts {
                let k = parts.0.to_string().clone().to_owned();
                let v = parts.1.to_string().clone().to_owned();
                form = form.text(k, v);
            }
        }

        // for uploading file
        for parts in &arg.mp_file {
            let (filename, filepath) = parts;
            let file = tfs::File::open(&filepath).await?;
            let stream = FramedRead::new(file, BytesCodec::new());
            let file_body = Body::wrap_stream(stream);

            // get file mime information
            let mime = mime_guess::from_path(filepath);
            let mime = mime.first_or_octet_stream();
            let mime = format!(
                "{}/{}",
                mime.type_().as_ref(),
                mime.subtype().as_ref()
            );

            form = form.part(
                filename.clone(),
                multipart::Part::stream(file_body)
                    .file_name(filename.clone())
                    .mime_str(mime.as_str())?,
            );
        }
        builder = builder.multipart(form);
    }

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::arg::{Method, OutputFormat};

    #[tokio::test]
    async fn test_set_request_text_body_with_body() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_text_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                text_body: Some("test body".to_string()),
                text_file: None,
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                json_file: None,
                json_body: None,
                json_command: None,
                form: vec![],
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_text_body_without_body() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_text_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                text_body: None,
                text_file: None,
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                json_file: None,
                json_body: None,
                json_command: None,
                form: vec![],
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_json_body_with_body() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_json_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                json_body: Some("{\"key\":\"value\"}".to_string()),
                json_file: None,
                json_command: None,
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                form: vec![],
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_json_body_without_body() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_json_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                json_body: None,
                json_file: None,
                json_command: None,
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                form: vec![],
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_form_body_with_params() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_form_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                form: vec![
                    "key1:value1".to_string(),
                    "key2:value2".to_string(),
                ],
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                json_file: None,
                json_body: None,
                json_command: None,
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_form_body_without_params() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_form_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                form: vec![],
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                json_file: None,
                json_body: None,
                json_command: None,
                mp: vec![],
                mp_file: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_multipart_body_with_params() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_multipart_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                mp: vec!["key1:value1".to_string()],
                mp_file: vec![],
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                json_file: None,
                json_body: None,
                json_command: None,
                form: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_request_multipart_body_without_params() {
        let client = Client::new();
        let builder =
            client.request(reqwest::Method::GET, "http://example.com");
        let result = set_request_multipart_body(
            &Arg {
                url: Some("http://example.com".to_string()),
                mp: vec![],
                mp_file: vec![],
                connections: 1,
                timeout: Duration::from_secs(30),
                latencies: false,
                percentiles: vec![],
                method: Method::Get,
                disable_keep_alive: false,
                headers: vec![],
                requests: Some(10),
                duration: None,
                rate: None,
                cert: None,
                key: None,
                insecure: false,
                text_file: None,
                text_body: None,
                json_file: None,
                json_body: None,
                json_command: None,
                form: vec![],
                output_format: OutputFormat::Text,
                completions: None,
            },
            builder,
        )
        .await;
        assert!(result.is_ok());
    }
}
