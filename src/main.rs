use autoclap::autoclap;
use clap::Arg;
use clap::Command;
use std::{env, io};

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() {
    let (input, output, silent, expected_sha256) = parse_args().await.expect("Cannot parse args");
    match aim::driver::Driver::drive(&input, &output, silent, &expected_sha256).await {
        Ok(_) => std::process::exit(0),
        _ => std::process::exit(255),
    }
}

#[cfg(not(tarpaulin_include))]
async fn parse_args() -> io::Result<(String, String, bool, String)> {
    let mut app: clap::Command = autoclap!()
        .arg(
            Arg::new("INPUT")
                .help(
                    "Input to aim from.\n\
                If no output provided and input is a folder, it will be served via http.",
                )
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("OUTPUT")
                .help(
                    "Explicit output to aim to. \n\
            * If no output argument is present, writes to stdout.\n\
            * Downloading: if file supplied, writes to it.\n\
              \x20\x20* if output is '.': downloads to the same basename as the source.\n\
              \x20\x20* if output is '+': downloads to the same basename as the source \n\
              \x20\x20\x20\x20and attempts to decompress the archive based on the file's extension.\n\
            * Uploading: directly uploads file to the URL.",
                )
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("SHA256")
                .help("Expected sha256 for verification. Will return a non-zero if mismatch.")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("silent")
                .long("silent")
                .short('s')
                .help("Silent or quiet mode. Don't show progress meter or error messages.")
                .required(false),
        )
        .arg(
            Arg::new("update")
                .long("update")
                .short('u')
                .help("Update the executable in-place.")
                .required(false),
        );
    let args = app.clone().try_get_matches().unwrap_or_else(|e| e.exit());

    if args.is_present("update") {
        tokio::task::spawn_blocking(move || match update() {
            Err(e) => {
                println!("ERROR: {}", e);
                ::std::process::exit(1);
            }
            Ok(()) => ::std::process::exit(0),
        })
        .await
        .unwrap();
    }
    let input = args.value_of("INPUT").unwrap_or_else(|| {
        app.print_help().unwrap();
        ::std::process::exit(0)
    });
    let output = args.value_of("OUTPUT").unwrap_or("stdout");
    let silent = args.is_present("silent");
    let expected_sha256 = args.value_of("SHA256").unwrap_or("");

    Ok((
        input.to_string(),
        output.to_string(),
        silent,
        expected_sha256.to_string(),
    ))
}

#[cfg(not(tarpaulin_include))]
fn update() -> Result<(), Box<dyn ::std::error::Error>> {
    let _status = self_update::backends::github::Update::configure()
        .repo_owner("mihaigalos")
        .repo_name(env!("CARGO_PKG_NAME"))
        .bin_name(env!("CARGO_PKG_NAME"))
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()?
        .update()?;
    println!("✅ Done.");
    Ok(())
}
