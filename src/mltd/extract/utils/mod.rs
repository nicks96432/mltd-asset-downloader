pub mod audio;
mod puzzle;
mod read_ext;

use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub use self::puzzle::*;
pub use self::read_ext::*;

#[cfg(not(feature = "debug"))]
pub fn ffmpeg<P>(
    input: &[u8],
    threads: usize,
    args: &str,
    output_path: P,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    #[rustfmt::skip]
    let ffmpeg_args = [
        "-loglevel quiet",
        "-threads", &threads.to_string(),
        "-i", "-",
        "-y",
    ];

    let ffmpeg_args = ffmpeg_args.into_iter().chain(args.split_ascii_whitespace());

    let mut ffmpeg = Command::new("ffmpeg")
        .args(ffmpeg_args)
        .arg(output_path.as_ref())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = ffmpeg.stdin.take().unwrap();
    stdin.write_all(input)?;
    drop(stdin);

    ffmpeg.wait()?;

    Ok(())
}

#[cfg(feature = "debug")]
pub fn ffmpeg<P>(
    input: &[u8],
    threads: usize,
    args: &str,
    _output_path: P,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    #[rustfmt::skip]
    let ffmpeg_args = [
        "-loglevel quiet",
        "-threads", &threads.to_string(),
        "-i", "-",
        "-y",
        "-f", "data",
        "-",
    ];

    let ffmpeg_args = ffmpeg_args.into_iter().chain(args.split_ascii_whitespace());

    let mut ffmpeg = Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = ffmpeg.stdin.take().unwrap();
    stdin.write_all(input)?;
    drop(stdin);

    ffmpeg.wait()?;

    Ok(())
}
