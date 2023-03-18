use std::fs as sfs;

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
    Client,
};

use crate::Arg;

pub(crate) fn build_client(arg: &Arg) -> anyhow::Result<Client> {
    let mut builder = Client::builder();

    // build headers
    let mut headers = HeaderMap::new();
    for header in &arg.headers {
        let parts = header.trim().split_once(':');
        if let Some(parts) = parts {
            headers.insert(
                HeaderName::from_bytes(parts.0.as_bytes())?,
                HeaderValue::from_str(parts.1)?,
            );
        }
    }

    // disable http keep alive
    if arg.disable_keep_alive {
        headers.insert("Connection", HeaderValue::from_static("Close"));
    }

    builder = builder
        .default_headers(headers)
        .timeout(arg.timeout)
        .connect_timeout(arg.timeout)
        .danger_accept_invalid_certs(arg.insecure)
        .danger_accept_invalid_hostnames(arg.insecure);

    // use client certificates
    if let Some(cert) = &arg.cert {
        if let Some(key) = &arg.key {
            let cert = sfs::read(cert)?;
            let key = sfs::read(key)?;
            let pkcs8 = reqwest::Identity::from_pkcs8_pem(&cert, &key)?;
            builder = builder.identity(pkcs8);
        }
    }

    // forbidden redirect
    builder = builder.redirect(Policy::none());

    match builder.build() {
        Ok(client) => Ok(client),
        Err(e) => Err(Box::new(e).into()),
    }
}
