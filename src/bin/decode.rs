use std::{
    env,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use filekit::{
    decode_file,
    decode_reader,
    write_new_file,
    write_new_file_with_writer,
};

use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let source = env::args_os()
        .nth(1)
        .ok_or("Usage: decode <input-file>")?;

    let source = PathBuf::from(source);

    if !source.is_file() {
        return Err("The input path is not a file.".into());
    }

    // Remove only our own suffix to determine the default name.
    let output = match source.to_str() {
        Some(name) if name.ends_with(".fkt") => {
            PathBuf::from(&name[..name.len() - 4])
        }
        _ => {
            let mut name = source.as_os_str().to_os_string();
            name.push(".restored");
            PathBuf::from(name)
        }
    };

    if output.exists() {
        return Err(
            format!("Output already exists: {}", output.display()).into()
        );
    }

    let passphrase = Zeroizing::new(
    rpassword::prompt_password("Passphrase: ")?
    );

    if passphrase.is_empty() {
        return Err("The passphrase cannot be empty.".into());
    }

    // Read only the four-byte format marker first.
    let mut input = File::open(&source)?;
    let mut magic = [0u8; 4];

    input.read_exact(&mut magic)?;

    match &magic {
        b"FKT1" => {
            println!("Restoring legacy FKT1 file...");

            // FKT1 is not a streaming format in our current
            // implementation, so it still loads the file
            // into memory.
            let protected = fs::read(&source)?;
            let original = Zeroizing::new(decode_file(&protected, &passphrase)?);

            write_new_file(&output, &original)?;
        }

        b"FKT2" => {
            println!("Restoring FKT2 file using streaming decryption...");

            // Reopen the source so the streaming decoder receives
            // the complete header, including the magic bytes.
            let mut input = File::open(&source)?;

            // Do not publish the restored file until the decoder
            // has authenticated every record and the final marker.
            write_new_file_with_writer(&output, |temporary| {
                decode_reader(&mut input, temporary, &passphrase)
                    .map_err(|error| {
                        std::io::Error::other(error.to_string())
                    })
            })?;
        }

        _ => {
            return Err("The file format is not recognised.".into());
        }
    }

    println!("Restored: {}", output.display());

    Ok(())
}