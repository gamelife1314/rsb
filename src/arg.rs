//! arg module define the application entry arguments [Arg]

use std::path::PathBuf;
use std::time::Duration;

use clap::{
    builder::{
        IntoResettable, OsStr, PossibleValue,
        Resettable::{self, *},
    },
    ArgGroup, Parser, ValueEnum, ValueHint,
};
use clap_complete::Shell;

fn is_number(s: &str) -> bool {
    s.parse::<u64>().is_ok()
}

fn parse_duration(arg: &str) -> Result<Duration, std::num::ParseIntError> {
    if is_number(arg) {
        return Ok(Duration::from_secs(arg.parse()?));
    }

    let mut input = arg;
    if input.ends_with("s") {
        input = &arg[..arg.len() - 1]
    }

    let seconds = input.parse()?;
    Ok(Duration::from_secs(seconds))
}

fn parse_percentiles(arg: &str) -> anyhow::Result<f32> {
    let value = arg.parse::<f32>()?;
    if value <= 0f32 || value >= 1f32 {
        anyhow::bail!("{} must be limited to the range (0 to 1)", value);
    }
    Ok(value)
}

fn parse_filename_and_path(s: &str) -> anyhow::Result<(String, PathBuf)> {
    let pos = s.find(':').ok_or(anyhow::anyhow!(
        "invalid filename:filepath,: no `:` found in `{s}`"
    ))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

/// define output format
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// Text output format
    Text,
    /// Json output format
    Json,
}

impl IntoResettable<OsStr> for OutputFormat {
    fn into_resettable(self) -> Resettable<OsStr> {
        match self {
            OutputFormat::Text => Value(OsStr::from("TEXT")),
            OutputFormat::Json => Value(OsStr::from("JSON")),
        }
    }
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[OutputFormat::Text, OutputFormat::Json]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            OutputFormat::Text => PossibleValue::new("TEXT"),
            OutputFormat::Json => PossibleValue::new("JSON"),
        })
    }
}

/// define supported http methods
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Method {
    /// Get request
    Get,
    /// Post request
    Post,
    /// put request
    Put,
    /// delete request
    Delete,
    /// head request
    Head,
    /// patch request
    Patch,
}

impl Method {
    /// convert to [reqwest::Method]
    pub(crate) fn to_reqwest_method(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
            Method::Head => reqwest::Method::HEAD,
        }
    }
}

impl IntoResettable<OsStr> for Method {
    fn into_resettable(self) -> Resettable<OsStr> {
        match self {
            Method::Get => Value(OsStr::from("GET")),
            Method::Post => Value(OsStr::from("POST")),
            Method::Put => Value(OsStr::from("PUT")),
            Method::Delete => Value(OsStr::from("DELETE")),
            Method::Head => Value(OsStr::from("HEAD")),
            Method::Patch => Value(OsStr::from("PATCH")),
        }
    }
}

impl ValueEnum for Method {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Method::Get,
            Method::Put,
            Method::Post,
            Method::Delete,
            Method::Head,
            Method::Patch,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            Method::Get => PossibleValue::new("GET"),
            Method::Put => PossibleValue::new("PUT"),
            Method::Post => PossibleValue::new("POST"),
            Method::Delete => PossibleValue::new("DELETE"),
            Method::Head => PossibleValue::new("HEAD"),
            Method::Patch => PossibleValue::new("PATCH"),
        })
    }
}

/// a http server benchmark tool, written in rust
#[derive(Debug, Parser)]
#[clap(color = concolor_clap::color_choice())]
#[command(author, version, about, allow_missing_positional(true))]
#[command(group(ArgGroup::new("json").args(["json_body", "json_file"])))]
#[command(group(ArgGroup::new("text").args(["text_body", "text_file"])))]
#[command(group(ArgGroup::new("multipart").args(["mp", "mp_file"]).multiple(true)))]
#[command(group(ArgGroup::new("mode").args(["duration", "requests"])))]
#[command(help_template(
    "\
{before-help}{name}({version}){tab}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}\
"
))]
pub struct Arg {
    /// Maximum number of concurrent connections
    #[arg(
        long,
        short,
        default_value_t = 50,
        help = "Maximum number of concurrent connections"
    )]
    pub connections: u16,

    /// Socket/request timeout
    #[arg(
        long,
        short,
        value_parser = parse_duration,
        default_value = "30s",
        help = "Socket/request timeout"
    )]
    pub(crate) timeout: Duration,

    /// Print latency statistics
    #[arg(long, short, help = "Print latency statistics")]
    pub(crate) latencies: bool,

    /// custom latency percentiles
    #[arg(
        long,
        num_args = 0..,
        value_parser = parse_percentiles,
        value_delimiter = ',',
        default_value = "0.5,0.75,0.9,0.99",
        help = "Custom latency percentiles"
    )]
    pub(crate) percentiles: Vec<f32>,

    /// Request method
    #[arg(
        long,
        short,
        default_value = Method::Get,
        value_enum,
        help = "Request method"
    )]
    pub method: Method,

    /// Disable HTTP keep-alive
    #[arg(long, short = 'a', help = "Disable HTTP keep-alive")]
    pub(crate) disable_keep_alive: bool,

    #[arg(
        long,
        short = 'H',
        num_args = 0..,
        value_delimiter = ',',
        help = "HTTP headers to use, example: -H=k:v,k1:v1 -H=k2:v2"
    )]
    pub(crate) headers: Vec<String>,

    /// Number of requests
    #[arg(
        long,
        short = 'n',
        help = "Number of requests",
        required_unless_present_any(["duration", "completions"])
    )]
    pub requests: Option<u64>,

    /// Duration of test
    #[arg(
        long,
        short = 'd',
        value_parser = parse_duration,
        help = "Duration of test",
        required_unless_present_any(["requests", "completions"])
    )]
    pub duration: Option<Duration>,

    /// Rate limit in requests per second
    #[arg(long, short = 'r', help = "Rate limit in requests per second")]
    pub(crate) rate: Option<u16>,

    /// Path to the client's TLS Certificate
    #[arg(
        long,
        value_hint = ValueHint::FilePath,
        requires("key"),
        help = "Path to the client's TLS Certificate"
    )]
    pub(crate) cert: Option<PathBuf>,

    /// Path to the client's TLS Certificate Private Key
    #[arg(
        long,
        requires("cert"),
        value_hint = ValueHint::FilePath,
        help = "Path to the client's TLS Certificate Private Key"
    )]
    pub(crate) key: Option<PathBuf>,

    /// Controls whether a client verifies the server's
    /// certificate chain and host name
    #[arg(
        long,
        short = 'k',
        help = "Controls whether a client verifies the server's certificate chain and host name"
    )]
    pub(crate) insecure: bool,

    /// File to use as json request body
    #[arg(
        long,
        value_hint = ValueHint::FilePath,
        conflicts_with_all(["mp_file", "mp", "form", "text_body", "text_file", "json_body"]),
        help = "File to use as Request body for ContentType: application/json"
    )]
    pub(crate) json_file: Option<PathBuf>,

    /// JSON request body
    #[arg(
        long,
        conflicts_with_all(["mp_file", "mp", "form", "text_body", "text_file", "json_file"]),
        help = "Request body for ContentType: application/json")]
    pub(crate) json_body: Option<String>,

    /// File to use as text request Body
    #[arg(
        long,
        value_hint = ValueHint::FilePath,
        conflicts_with_all(["mp_file", "mp", "form", "text_body", "json_file", "json_body"]),
        help = "File to use as Request body for ContentType: text/plain"
    )]
    pub(crate) text_file: Option<PathBuf>,

    /// Text request body
    #[arg(
        long,
        conflicts_with_all(["mp_file", "mp", "form", "text_file", "json_file", "json_body"]),
        help = "Request body for ContentType: text/plain"
    )]
    pub(crate) text_body: Option<String>,

    /// multipart body parameters
    #[arg(
        long,
        num_args = 0..,
        value_delimiter = ',',
        conflicts_with_all(["form", "text_body", "text_file", "json_file", "json_body"]),
        help = "Multipart body parameters, Content-Type: multipart/form-data, example: --mp=k1:v1,k2:v2"
    )]
    pub(crate) mp: Vec<String>,

    /// multipart body file
    #[arg(
        long,
        num_args = 0..,
        value_delimiter = ',',
        value_parser = parse_filename_and_path,
        conflicts_with_all(["form", "text_body", "text_file", "json_file", "json_body"]),
        help = "Multipart body files, Content-Type: multipart/form-data, example: --mp-file=t1:t1.txt,filename2:t2.txt"
    )]
    pub(crate) mp_file: Vec<(String, PathBuf)>,

    /// form request parameters
    #[arg(
        long,
        num_args = 0..,
        value_delimiter = ',',
        conflicts_with_all(["mp_file", "mp", "text_body", "text_file", "json_file", "json_file"]),
        help = "Form request parameters, Content-Type: application/x-www-form-urlencoded, example: --form=k:v,k1:v1"
    )]
    pub(crate) form: Vec<String>,

    /// Output Format
    #[arg(
        long,
        default_value = OutputFormat::Text,
        value_enum,
        help = "Output format"
    )]
    pub output_format: OutputFormat,

    /// for shell autocompletion, supports: bash, shell, powershell, zsh and
    /// elvish
    #[arg(long, value_enum)]
    pub completions: Option<Shell>,

    /// Target Url
    #[arg(
        required_unless_present("completions"), 
        value_hint = ValueHint::Url,
        help = "Target Url"
    )]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::{Command, CommandFactory};

    use super::*;
    const URI: &str = "https://localhost/test";
    const BINARY: &str = "rsb";

    #[test]
    fn test_is_number() {
        assert!(is_number("12234"));
        assert!(!is_number("12234s"));
    }

    #[test]
    fn test_parse_duration() {
        assert!(parse_duration("123").is_ok());
        assert!(parse_duration("123s").is_ok());
        assert!(parse_duration("123x").is_err());
    }

    #[test]
    fn test_method_choices() {
        let mut cmd = Arg::command();
        let methods = vec!["GET", "PUT", "POST", "DELETE", "HEAD", "PATCH"];
        for method in methods {
            let args = vec![BINARY, "-n", "20", "-m", method, URI];
            let result = cmd.try_get_matches_from_mut(args);
            assert!(result.is_ok());
        }

        let result = cmd.try_get_matches_from_mut(vec![
            BINARY, "-n", "20", "-m", "get", URI,
        ]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg
            .contains("possible values: GET, PUT, POST, DELETE, HEAD, PATCH"));
    }

    #[test]
    fn test_required_parameters() {
        let mut cmd = Arg::command();
        let result = cmd.try_get_matches_from_mut(vec!["rsb"]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains(
            "error: the following required arguments were not provided:
  --requests <REQUESTS>
  --duration <DURATION>
  <URL>"
        ))
    }

    #[test]
    fn test_must_provide_requests_or_duration_parameters() {
        let mut cmd = Arg::command();

        // neither provided
        let result = cmd.try_get_matches_from_mut(vec![BINARY, URI]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains(
            "error: the following required arguments were not provided:
  --requests <REQUESTS>
  --duration <DURATION>"
        ));

        // both provide
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY, "-n", "20", "-d", "300", URI,
        ]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains(
            "error: the argument '--requests <REQUESTS>' \
        cannot be used with '--duration <DURATION>'"
        ));
    }

    #[test]
    fn test_require_key_and_cert_at_the_same_time() {
        let mut cmd = Arg::command();

        // only provide key
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY, "-n", "20", "--key", "key.crt", URI,
        ]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains(
            "error: the following required arguments were not provided:
  --cert <CERT>"
        ));

        // only provide cert
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY, "-n", "20", "--cert", "cert.crt", URI,
        ]);
        assert!(result.as_ref().is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains(
            "error: the following required arguments were not provided:
  --key <KEY>"
        ));

        // both provided
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY, "-n", "20", "--cert", "cert.crt", "--key", "key.crt", URI,
        ]);
        assert!(result.as_ref().is_ok());
    }

    #[test]
    fn test_json_body_or_json_file() {
        let mut cmd = Arg::command();

        // both provided
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--json-body",
            "jsonBody",
            "--json-file",
            "jsonBody.txt",
            URI,
        ]);
        assert!(result.as_ref().is_err());

        // only json body
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--json-body",
            "jsonBody",
            URI,
        ]);
        assert!(result.as_ref().is_ok());

        // only json body
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--json-file",
            "jsonBody.txt",
            URI,
        ]);
        assert!(result.as_ref().is_ok());
    }

    #[test]
    fn test_json_body_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args =
            vec![BINARY, "-n", "20", "--json-body", "jb", "xx", "xx", URI];
        let conflicts_params = vec![
            "--mp-file",
            "--mp",
            "--form",
            "--text-body",
            "--text-file",
            "--json-file",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    #[test]
    fn test_json_file_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args =
            vec![BINARY, "-n", "20", "--json-file", "jb", "xx", "xx", URI];
        let conflicts_params = vec![
            "--mp-file",
            "--mp",
            "--form",
            "--text-body",
            "--text-file",
            "--json-body",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    #[test]
    fn test_text_body_or_text_file() {
        let mut cmd = Arg::command();

        // both provided
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--text-body",
            "text",
            "--text-file",
            "ts.txt",
            URI,
        ]);
        assert!(result.as_ref().is_err());

        // only json body
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--text-body",
            "text",
            URI,
        ]);
        assert!(result.as_ref().is_ok());

        // only json body
        let result = cmd.try_get_matches_from_mut(vec![
            BINARY,
            "-n",
            "20",
            "--text-file",
            "jsonBody.txt",
            URI,
        ]);
        assert!(result.as_ref().is_ok());
    }

    #[test]
    fn test_text_body_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args =
            vec![BINARY, "-n", "20", "--text-body", "jb", "xx", "xx", URI];
        let conflicts_params = vec![
            "--mp-file",
            "--mp",
            "--form",
            "--json-body",
            "--text-file",
            "--json-file",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    fn validate_args_conflict<'a>(
        conflicts_params: Vec<&'a str>,
        mut args: Vec<&'a str>,
        mut cmd: Command,
    ) {
        for cp in conflicts_params {
            args[5] = cp;
            if cp == "--mp-file" {
                args[6] = "filename1:file2.txt";
            }
            let result = cmd.try_get_matches_from_mut(args.clone());
            assert!(result.as_ref().is_err());
            let err_msg = result.err().unwrap().to_string();
            let fragment = format!("cannot be used with '{cp}");
            assert!(err_msg.contains(&fragment));
        }
    }

    #[test]
    fn test_text_file_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args =
            vec![BINARY, "-n", "20", "--text-file", "jb", "xx", "xx", URI];
        let conflicts_params = vec![
            "--mp-file",
            "--mp",
            "--form",
            "--text-body",
            "--json-file",
            "--json-body",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    #[test]
    fn test_multipart_param_exist_with_multipart_file() {
        let mut cmd = Arg::command();
        let args = vec![
            BINARY,
            "-n",
            "20",
            "--mp=k1:v1,k2:v2",
            "--mp-file=t1:text1.txt,t2:text2.txt",
            URI,
        ];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let mps = result
            .as_ref()
            .unwrap()
            .get_many::<String>("mp")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(mps, ["k1:v1", "k2:v2"]);

        let mp_files = result
            .as_ref()
            .unwrap()
            .get_many::<(String, PathBuf)>("mp_file")
            .unwrap()
            .collect::<Vec<_>>();
        let mp_files = mp_files
            .iter()
            .map(|x| (x.0.as_str(), x.1.to_str().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(mp_files, [("t1", "text1.txt"), ("t2", "text2.txt")]);
    }

    #[test]
    fn test_mp_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args = vec![
            BINARY,
            "-n",
            "20",
            "--mp=k1:v1,k2:v2",
            "--mp-file=text1:text1.txt",
            "xx",
            "xx",
            URI,
        ];
        let conflicts_params = vec![
            "--text-file",
            "--form",
            "--text-body",
            "--json-file",
            "--json-body",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    #[test]
    fn test_parse_form_params() {
        let mut cmd = Arg::command();

        // style1
        let args = vec![BINARY, "-n", "20", "--form=k1:v1,k2:v2", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let forms = result
            .as_ref()
            .unwrap()
            .get_many::<String>("form")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(forms, ["k1:v1", "k2:v2"]);

        // style2
        let args =
            vec![BINARY, "-n", "20", "--form=k1:v1", "--form=k2:v2", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let forms = result
            .as_ref()
            .unwrap()
            .get_many::<String>("form")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(forms, ["k1:v1", "k2:v2"]);
    }

    #[test]
    fn test_form_conflicts_with_other_body() {
        let cmd = Arg::command();
        let args = vec![
            BINARY,
            "-n",
            "20",
            "--form=k1:v1,k2:v2",
            "--form=k3:v3,k4:v4",
            "xx",
            "xx",
            URI,
        ];
        let conflicts_params = vec![
            "--text-file",
            "--mp-file",
            "--mp",
            "--text-body",
            "--json-file",
            "--json-body",
        ];
        validate_args_conflict(conflicts_params, args, cmd);
    }

    #[test]
    fn test_parse_headers_params() {
        let mut cmd = Arg::command();

        // style1
        let args =
            vec![BINARY, "-n", "20", "-H=k1:v1,k2:v2", "--headers=k3:v3", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let headers = result
            .as_ref()
            .unwrap()
            .get_many::<String>("headers")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(headers, ["k1:v1", "k2:v2", "k3:v3"]);
    }

    #[test]
    fn test_parse_percentiles_params() {
        let mut cmd = Arg::command();

        // default percentiles
        let args = vec![BINARY, "-n", "20", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let percentiles = result
            .as_ref()
            .unwrap()
            .get_many::<f32>("percentiles")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(format!("{percentiles:?}"), "[0.5, 0.75, 0.9, 0.99]");

        // custom percentiles
        let args =
            vec![BINARY, "-n", "20", "--percentiles=0.60,0.80,0.90,0.95", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());
        let percentiles = result
            .as_ref()
            .unwrap()
            .get_many::<f32>("percentiles")
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(format!("{percentiles:?}"), "[0.6, 0.8, 0.9, 0.95]");

        // given wrong percent, negative
        let args = vec![BINARY, "-n", "20", "--percentiles=-0.1", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("-0.1 must be limited to the range (0 to 1)"));

        // give wrong percent, that's gte 1
        let args = vec![BINARY, "-n", "20", "--percentiles=1", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("1 must be limited to the range (0 to 1)"));
    }

    #[test]
    fn test_parse_output_format() {
        let mut cmd = Arg::command();

        // TEXT
        let args = vec![BINARY, "-n", "20", "--output-format", "TEXT", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        // JSON
        let args = vec![BINARY, "-n", "20", "--output-format", "JSON", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_ok());

        // OTHER
        let args = vec![BINARY, "-n", "20", "--output-format", "OTHER", URI];
        let result = cmd.try_get_matches_from_mut(args);
        assert!(result.as_ref().is_err());
    }
}
