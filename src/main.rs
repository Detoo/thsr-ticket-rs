use reqwest::blocking::Client;
use std::{fs::File, io::{self, Write}};
use std::path::Path;
use opener;

fn download_captcha(client: &Client, url: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url).send()?;
    let bytes = response.bytes()?;

    let mut file = File::create(output_path)?;
    file.write_all(&bytes)?;

    println!("Downloaded CAPTCHA image to: {}", output_path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;
    let captcha_url = "https://example.com/captcha";
    let captcha_path = Path::new("tmp/captcha.png");

    // TODO WIP: Download CAPTCHA image
    // download_captcha(&client, captcha_url, captcha_path)?;

    // Open the image for the user
    println!("Opening {}... Please view and type the CAPTCHA shown.", captcha_path.display());
    opener::open(captcha_path)?;

    // Get user input for CAPTCHA
    let mut captcha_solution = String::new();
    io::stdin().read_line(&mut captcha_solution)?;
    let captcha_solution = captcha_solution.trim();

    println!("CAPTCHA solution entered: {}", captcha_solution);

    Ok(())
}
