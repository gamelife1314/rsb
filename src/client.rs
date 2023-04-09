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

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsStr;
    use std::path::Path;

    use clap::{CommandFactory, FromArgMatches};

    use super::*;
    const URI: &str = "https://localhost/test";
    const BINARY: &str = "rsb";

    #[test]
    fn test_build_client_success() {
        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_ok());
    }

    #[test]
    fn test_build_client_with_custom_headers_success() {
        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", "--headers=k3:v3", URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_ok());
    }

    #[test]
    fn test_build_client_with_invalid_custom_headers_success() {
        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", "--headers=k3", URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_ok());
    }

    #[test]
    fn test_build_client_disable_keep_alive() {
        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", "-a", URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_ok());
    }

    #[test]
    fn test_build_client_with_client_cert() {
        let root = env::var("CARGO_MANIFEST_DIR").unwrap();
        let tests = Path::new(OsStr::new(root.as_str()))
            .join("resources")
            .join("tests");
        let binding = tests.join("client.pem");
        let cert = binding.to_str().unwrap();

        let binding = tests.join("client-key.pem");
        let key = binding.to_str().unwrap();

        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", "--cert", cert, "--key", key, URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_ok());
    }

    #[test]
    fn test_build_client_with_invalid_client_cert() {
        let cert = file!();
        let key = file!();

        let mut cmd = Arg::command();
        let args = vec![BINARY, "-n", "20", "--cert", cert, "--key", key, URI];
        let mut result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        let arg = Arg::from_arg_matches_mut(result.as_mut().unwrap()).unwrap();
        let client = build_client(&arg);
        assert!(client.as_ref().is_err());
    }
}
