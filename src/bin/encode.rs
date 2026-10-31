use std::{
    env,
    fs::File,
    path::PathBuf,
};

use filekit::{encode_reader, write_new_file_with_writer};
use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Read the source path from the command line.
    let source = env::args_os()
        .nth(1)
        .ok_or("Usage: encode <input-file>")?;

    let source = PathBuf::from(source);

    if !source.is_file() {
        return Err("The input path is not a file.".into());
    }

    // Use a neutral suffix for the output file.
    let mut output_name = source.as_os_str().to_os_string();
    output_name.push(".fkt");
    let output = PathBuf::from(output_name);

    // Never overwrite a file without explicit user action.
    if output.exists() {
        return Err(format!("Output already exists: {}", output.display()).into());
    }

    // Prompt without echoing the passphrase to the terminal.
    let passphrase = Zeroizing::new(
    rpassword::prompt_password("Passphrase: ")?
	);

    if passphrase.is_empty() {
        return Err("The passphrase cannot be empty.".into());
    }

    println!("Encrypting file using FKT2 streaming format...");

    // Open the input file without reading it all into memory.
    let mut input = File::open(&source)?;

    // Write to a temporary file. Publish the final destination
    // only if streaming encryption completes successfully.
    write_new_file_with_writer(&output, |temporary| {
        encode_reader(&mut input, temporary, &passphrase)
            .map_err(|error| std::io::Error::other(error.to_string()))
    })?;

    println!("Saved: {}", output.display());

    Ok(())
}