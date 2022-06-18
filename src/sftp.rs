extern crate ssh2;

use async_io::Async;
use async_ssh2_lite::AsyncSession;
use futures::executor::block_on;
use futures::AsyncReadExt;
use futures::AsyncSeekExt;
use futures::AsyncWriteExt;
use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::address::ParsedAddress;
use crate::bar::WrappedBar;
use crate::consts::*;
use crate::error::ValidateError;
use crate::hash::HashChecker;
use crate::io::get_output;
use crate::ssh_auth::get_possible_ssh_keys_path;

pub struct SFTPHandler;
impl SFTPHandler {
    pub async fn get(
        input: &str,
        output: &str,
        bar: &mut WrappedBar,
        expected_sha256: &str,
    ) -> Result<(), ValidateError> {
        SFTPHandler::_get(input, output, bar).await?;
        HashChecker::check(output, expected_sha256)
    }
    async fn _get(input: &str, output: &str, bar: &mut WrappedBar) -> Result<(), ValidateError> {
        let (session, remote_file) = SFTPHandler::setup_session(input, bar.silent).await;
        let (mut out, mut transfered) = get_output(output, bar.silent);
        // let (mut remote_file, stat) = session.scp_recv(Path::new(&remote_file)).await.unwrap();
        let sftp = session.sftp().await.unwrap();
        let stat = sftp
            .stat(Path::new(&remote_file))
            .await
            .expect("Cannot stat remote SFTP file");
        let mut remote_file = sftp
            .open(Path::new(&remote_file))
            .await
            .expect("Cannot open remote SFTP file");
        let total_size = stat.size.expect("Cannot get remote SFTP file size");
        bar.set_length(total_size);

        remote_file
            .seek(SeekFrom::Current(transfered as i64))
            .await
            .expect("Cannot seek in SFTP file");
        loop {
            let mut buffer = vec![0; BUFFER_SIZE];
            let byte_count = remote_file
                .read(&mut buffer)
                .await
                .expect("Cannot read SFTP stream");
            buffer.truncate(byte_count);
            if !buffer.is_empty() {
                out.write_all(&buffer)
                    .or(Err(format!("Error while writing to output")))
                    .unwrap();
                let new = min(transfered + (byte_count as u64), total_size);
                transfered = new;
                bar.set_position(new);
            } else {
                break;
            }
        }
        bar.finish_download(&input, &output);
        Ok(())
    }

    // pub async fn put(input: &str, output: &str, mut bar: WrappedBar) -> Result<(), ValidateError> {
    //     let mut remote_file = session
    //         .scp_send(Path::new(output), 0o644, 10, None)
    //         .await
    //         .unwrap();
    //     remote_file.write(b"1234567890").await.unwrap();

    // }
    async fn setup_session(
        address: &str,
        silent: bool,
    ) -> (async_ssh2_lite::AsyncSession<std::net::TcpStream>, String) {
        let parsed_address = ParsedAddress::parse_address(address, silent);

        let addr = parsed_address
            .server
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();
        let stream = Async::<TcpStream>::connect(addr).await.unwrap();
        let mut session = AsyncSession::new(stream, None).unwrap();
        session.handshake().await.expect("SFTP handshake failed");
        if parsed_address.password != "anonymous" {
            session
                .userauth_password(&parsed_address.username, &parsed_address.password)
                .await
                .expect("SFTP Authentication failed");
        } else {
            let ssh_keys = get_possible_ssh_keys_path(silent);
            let mut is_ok = false;
            for ssh_key in ssh_keys.iter() {
                if session
                    .userauth_pubkey_file(
                        &parsed_address.username,
                        Some(Path::new(&(ssh_key.to_owned() + ".pub"))),
                        Path::new(ssh_key),
                        None,
                    )
                    .await
                    .is_ok()
                {
                    is_ok = true;
                    break;
                }
            }

            if !is_ok {
                println!("SFTP Authentication failed. Please specifiy a user: sftp://user@address");
            }
        }

        let remote_file = String::from("/")
            + &parsed_address.path_segments.join("/")
            + "/"
            + &parsed_address.file;
        (session, remote_file)
    }
}
