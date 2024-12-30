use reqwest::blocking::Client;
use std::{fs::File, io::{self, Write}};
use std::path::Path;
use opener;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HOST, USER_AGENT};
use reqwest::redirect::Policy;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum Station {
    Nangang = 1,
    Taipei,
    Banqiao,
    Taoyuan,
    Hsinchu,
    Miaoli,
    Taichung,
    Changhua,
    Yunlin,
    Chiayi,
    Tainan,
    Zuouing,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Booking {
    #[serde(rename = "selectStartStation")]
    start_station: Station,
    #[serde(rename = "selectDestinationStation")]
    dest_station: Station,
    #[serde(rename = "bookingMethod")]
    search_by: String,
}

fn download_captcha(client: &Client, url: &str, headers: HeaderMap, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url).headers(headers).send()?;
    let bytes = response.bytes()?;

    let mut file = File::create(output_path)?;
    file.write_all(&bytes)?;

    println!("Downloaded CAPTCHA image to: {}", output_path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .redirect(Policy::default())
        .cookie_store(true)
        .build()?;
    let booking_page_url = "https://irs.thsrc.com.tw/IMINT/?locale=tw";
    let captcha_local_path = Path::new("tmp/captcha.png");

    // TODO test
    // // Show the CAPTCHA image
    // let mut headers = HeaderMap::new();
    // headers.insert(HOST, HeaderValue::from_static("irs.thsrc.com.tw"));
    // headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/12.246"));
    // headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"));
    // headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-TW,zh;q=0.8,en-US;q=0.5,en;q=0.3"));
    // headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    // let response = client
    //     .get(booking_page_url)
    //     .headers(headers.clone())
    //     .send()?.text()?;
    // let document = Html::parse_document(&response);
    // let selector = Selector::parse("#BookingS1Form_homeCaptcha_passCode").unwrap();
    // let element = document.select(&selector).next().expect("Couldn't find the captcha element");
    // let src = element.value().attr("src").expect("Couldn't find the captcha source url");
    // let captcha_url = ["https://irs.thsrc.com.tw", src].concat();
    // println!("Captcha url: {}", captcha_url);
    // download_captcha(&client, captcha_url.as_str(), headers, captcha_local_path)?;
    // opener::open(captcha_local_path)?;
    //
    // // Get user input for CAPTCHA
    // println!("Type the answer to the CAPTCHA: ");
    // let mut captcha_solution = String::new();
    // io::stdin().read_line(&mut captcha_solution)?;
    // let captcha_solution = captcha_solution.trim();
    // println!("CAPTCHA solution entered: {}", captcha_solution);

    // TODO Get booking parameters either from presets or user input
    // TODO Fake the booking parameters for now
    // TODO Use serde to model the booking parameters
    let booking = Booking{
        start_station: Station::Nangang,
        dest_station: Station::Zuouing,
        search_by: String::from("radio31"),
    };
    println!("booking: {:?}", booking);
    println!("booking (json): {}", serde_json::to_string(&booking).unwrap());

    Ok(())
}
