use std::collections::HashMap;

use bytes::Bytes;
use reqwest::{multipart, Body, Client, Request, RequestBuilder};
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
