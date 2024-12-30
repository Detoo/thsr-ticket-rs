use reqwest::blocking::Client;
use std::{fs::File, io::{self, Write}};
use std::path::Path;
use chrono::NaiveDateTime;
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

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum Trip {
    OneWay = 0,
    RoundTrip,
}

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
enum CabinClass {
    #[default]
    Standard = 0,
    Business,
}

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum SeatPref {
    NoPref = 0,
    Window,
    Aisle,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Booking {
    #[serde(rename = "selectStartStation")]
    start_station: Station,
    #[serde(rename = "selectDestinationStation")]
    dest_station: Station,
    #[serde(rename = "bookingMethod")]
    search_by: String,
    #[serde(rename = "tripCon:typesoftrip")]
    types_of_trip: Trip,

    // TODO Implement more sophisticated logic to serialize datetime as a chrono NaiveDateTime instance
    // outbound_datetime: NaiveDateTime,
    #[serde(rename = "toTimeInputField")]
    outbound_date: String,
    #[serde(rename = "toTimeTable")]
    outbound_time: String,

    #[serde(rename = "homeCaptcha:securityCode")]
    security_code: String,
    #[serde(rename = "seatCon:seatRadioGroup")]
    seat_prefer: SeatPref,
    #[serde(default, rename = "BookingS1Form:hf:0")]
    form_mark: String,
    #[serde(default, rename = "trainCon:trainRadioGroup")]
    class_type: CabinClass,

    // TODO Implement more sophisticated logic to serialize datetime as a chrono NaiveDateTime instance
    // inbound_datetime: NaiveDateTime,
    #[serde(default, rename = "backTimeInputField")]
    inbound_date: Option<String>,
    #[serde(default, rename = "backTimeTable")]
    inbound_time: Option<String>,

    #[serde(default, rename = "toTrainIDInputField")]
    to_train_id: Option<i16>,
    #[serde(default, rename = "backTrainIDInputField")]
    back_train_id: Option<i16>,

    // TODO There must be a better way to represent this
    #[serde(default = "default_adult_ticket", rename = "ticketPanel:rows:0:ticketAmount")]
    adult_ticket_num: String,
    #[serde(default = "default_child_ticket", rename = "ticketPanel:rows:1:ticketAmount")]
    child_ticket_num: String,
    #[serde(default = "default_disabled_ticket", rename = "ticketPanel:rows:2:ticketAmount")]
    disabled_ticket_num: String,
    #[serde(default = "default_elder_ticket", rename = "ticketPanel:rows:3:ticketAmount")]
    elder_ticket_num: String,
    #[serde(default = "default_college_ticket", rename = "ticketPanel:rows:4:ticketAmount")]
    college_ticket_num: String,
}

fn default_adult_ticket() -> String {
    "1F".to_string()
}

fn default_child_ticket() -> String {
    "0H".to_string()
}

fn default_disabled_ticket() -> String {
    "0W".to_string()
}

fn default_elder_ticket() -> String {
    "0E".to_string()
}

fn default_college_ticket() -> String {
    "0P".to_string()
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

    // Show the CAPTCHA image
    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("irs.thsrc.com.tw"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/12.246"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-TW,zh;q=0.8,en-US;q=0.5,en;q=0.3"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    let response = client
        .get(booking_page_url)
        .headers(headers.clone())
        .send()?;

    // Find session ID
    let session_id_cookie = response.cookies().find(|cookie| cookie.name() == "JSESSIONID").unwrap();
    println!("JSESSIONID: {}", session_id_cookie.value());

    let response_text = response.text()?;

    let document = Html::parse_document(&response_text);
    let selector = Selector::parse("#BookingS1Form_homeCaptcha_passCode").unwrap();
    let element = document.select(&selector).next().expect("Couldn't find the captcha element");
    let src = element.value().attr("src").expect("Couldn't find the captcha source url");
    let captcha_url = ["https://irs.thsrc.com.tw", src].concat();
    println!("Captcha url: {}", captcha_url);
    download_captcha(&client, captcha_url.as_str(), headers, captcha_local_path)?;
    opener::open(captcha_local_path)?;

    // // Get user input for CAPTCHA
    // println!("Type the answer to the CAPTCHA: ");
    // let mut captcha_solution = String::new();
    // io::stdin().read_line(&mut captcha_solution)?;
    // let captcha_solution = captcha_solution.trim();
    // println!("CAPTCHA solution entered: {}", captcha_solution);

    // TODO Get booking parameters either from presets or user input
    // TODO Fake the booking parameters for now
    // TODO Use serde to model the booking parameters

    // TODO test
    // let booking = Booking{
    //     start_station: Station::Nangang,
    //     dest_station: Station::Zuouing,
    //     search_by: String::from("radio31"),
    //     types_of_trip: Trip::OneWay,
    //     // TODO test
    //     // outbound_datetime: DateTime::parse_from_str("2025/01/21 10:00 AM", "%Y/%m/%d %H:%M")
    //     outbound_date: String::from("2025/01/21"),
    //     outbound_time: String::from("1000A"),
    //     security_code: String::from("abcd"),
    //     seat_prefer: SeatPref::Window,
    //     form_mark: String::from(""),
    //     class_type: CabinClass::Business,
    //     inbound_date: None,
    //     inbound_time: None,
    //     to_train_id: None,
    //     back_train_id: None,
    //     adult_ticket_num: String::from("1F"),
    //     child_ticket_num: String::from("0H"),
    //     disabled_ticket_num: String::from("0W"),
    //     elder_ticket_num: String::from("2E"),
    //     college_ticket_num: String::from("0P"),
    // };
    // println!("booking: {:?}", booking);
    // println!("booking (json): {}", serde_json::to_string(&booking).unwrap());
    //
    // // TODO test
    // // let dt = NaiveDateTime::parse_from_str("2025/01/27 22:00", "%Y/%m/%d %H:%M").unwrap();
    // // println!("datetime: {}", dt);
    //
    // // Submit booking form
    // let submit_booking_form_url = format!("https://irs.thsrc.com.tw/IMINT/;jsessionid={}?wicket:interface=:0:BookingS1Form::IFormSubmitListener", client.);

    Ok(())
}
