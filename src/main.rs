use reqwest::blocking::Client;
use std::{fmt, fs::File, io::{self, Write}};
use std::error::Error;
use std::path::Path;
use chrono::NaiveDateTime;
use opener;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HOST, USER_AGENT};
use reqwest::redirect::Policy;
use scraper::{ElementRef, Html, Selector};
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

#[derive(Debug)]
struct TrainInfo {
    id: i16,
    depart: String,
    arrive: String,
    travel_time: String,
    discount_str: String,
    form_value: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TrainSelection {
    #[serde(rename = "TrainQueryDataViewPanel:TrainGroup")]
    selected_train: String,
    #[serde(default, rename = "BookingS2Form:hf:0")]
    form_mark: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TicketConfirmation {
    #[serde(rename = "dummyId")]
    personal_id: String,
    #[serde(rename = "dummyPhone")]
    phone_num: String,
    #[serde(rename = "TicketMemberSystemInputPanel:TakerMemberSystemDataView:memberSystemRadioGroup")]
    member_radio: String,
    #[serde(default, rename = "BookingS3FormSP:hf:0")]
    form_mark: String,
    #[serde(default, rename = "idInputRadio")]
    id_input_radio: i8,
    #[serde(default = "default_1_i8", rename = "diffOver")]
    diff_over: i8,
    #[serde(default, rename = "email")]
    email: String,
    #[serde(default = "default_agree", rename = "agree")]
    agree: String,
    #[serde(default, rename = "isGoBackM")]
    go_back_m: String,
    #[serde(default, rename = "backHome")]
    back_home: String,
    #[serde(default = "default_1_i8", rename = "TgoError")]
    tgo_error: i8,
    // TODO Make it dynamic. Current implementation assumes 1 adult, 2 elder because the aliases are type and order dependent
    #[serde(default, rename = "TicketPassengerInfoInputPanel:passengerDataView:1:passengerDataView2:passengerDataIdNumber")]
    elder_id0: String,
    #[serde(default, rename = "TicketPassengerInfoInputPanel:passengerDataView:2:passengerDataView2:passengerDataIdNumber")]
    elder_id1: String,
}

fn default_1_i8() -> i8 {
    1
}

fn default_agree() -> String {
    "on".to_string()
}

#[derive(Debug)]
struct ErrorMessages {
    errors: Vec<String>,
}

impl fmt::Display for ErrorMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.errors)
    }
}

impl Error for ErrorMessages {}

fn download_captcha(client: &Client, url: &str, headers: HeaderMap, output_path: &Path) -> Result<(), Box<dyn Error>> {
    let response = client.get(url).headers(headers).send()?;
    let bytes = response.bytes()?;

    let mut file = File::create(output_path)?;
    file.write_all(&bytes)?;

    println!("Downloaded CAPTCHA image to: {}", output_path.display());
    Ok(())
}

fn gen_common_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("irs.thsrc.com.tw"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/12.246"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-TW,zh;q=0.8,en-US;q=0.5,en;q=0.3"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.clone()
}

fn parse_discount(item: ElementRef) -> String {
    let mut discounts: Vec<String> = Vec::new();
    if let Some(discount) = item.select(&Selector::parse("p.early-bird").unwrap()).next() {
        discounts.push(discount.inner_html());
    }
    if let Some(discount) = item.select(&Selector::parse("p.student").unwrap()).next() {
        discounts.push(discount.inner_html());
    }
    discounts.join(", ")
}

fn assert_submission_errors(response_text: String) -> Result<(), ErrorMessages> {
    let document = Html::parse_document(&response_text);
    let errors: Vec<String> = document
        .select(&Selector::parse("span.feedbackPanelERROR").unwrap())
        .filter_map(|element| element.text().next().map(|text| text.to_string()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ErrorMessages{ errors })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::builder()
        .redirect(Policy::default())
        .cookie_store(true)
        .build()?;
    let booking_page_url = "https://irs.thsrc.com.tw/IMINT/?locale=tw";
    let captcha_local_path = Path::new("tmp/captcha.png");

    // Start a new session
    let session_id = {
        let response = client
            .get(booking_page_url)
            .headers(gen_common_headers())
            .send()?;

        // Find session ID
        let session_id = response.cookies().find(|cookie| cookie.name() == "JSESSIONID").unwrap().value().to_string();

        // Show CAPTCHA image
        let response_text = response.text()?;
        let document = Html::parse_document(&response_text);
        let selector = Selector::parse("#BookingS1Form_homeCaptcha_passCode").unwrap();
        let element = document.select(&selector).next().expect("Couldn't find the captcha element");
        let src = element.value().attr("src").expect("Couldn't find the captcha source url");
        let captcha_url = ["https://irs.thsrc.com.tw", src].concat();
        println!("Captcha url: {}", captcha_url);
        download_captcha(&client, captcha_url.as_str(), gen_common_headers(), captcha_local_path)?;
        opener::open(captcha_local_path)?;

        session_id
    };
    println!("JSESSIONID: {}", session_id);

    // Get user input for CAPTCHA
    let captcha_solution = {
        println!("Type the answer to the CAPTCHA: ");
        let mut captcha_solution = String::new();
        io::stdin().read_line(&mut captcha_solution)?;
        captcha_solution.trim().to_string()
    };
    println!("CAPTCHA solution entered: {}", captcha_solution);

    // Prepare booking info
    let booking = {
        // TODO Get booking parameters either from presets or user input
        // TODO Fake the booking parameters for now

        // TODO test
        // let dt = NaiveDateTime::parse_from_str("2025/01/27 22:00", "%Y/%m/%d %H:%M").unwrap();
        // println!("datetime: {}", dt);

        // TODO Test booking
        Booking{
            start_station: Station::Nangang,
            dest_station: Station::Zuouing,
            search_by: String::from("radio31"),
            types_of_trip: Trip::OneWay,
            // TODO test
            // outbound_datetime: DateTime::parse_from_str("2025/01/21 10:00 AM", "%Y/%m/%d %H:%M")

            // TODO test
            outbound_date: String::from("2025/01/21"),
            // outbound_date: String::from("2025/02/21"),

            outbound_time: String::from("930A"),
            security_code: captcha_solution.to_string(),
            seat_prefer: SeatPref::Window,
            form_mark: String::from(""),
            class_type: CabinClass::Business,
            inbound_date: None,
            inbound_time: None,
            to_train_id: None,
            back_train_id: None,
            adult_ticket_num: String::from("1F"),
            child_ticket_num: String::from("0H"),
            disabled_ticket_num: String::from("0W"),
            elder_ticket_num: String::from("2E"),
            college_ticket_num: String::from("0P"),
        }
    };
    println!("booking: {:?}", booking);
    println!("booking (json): {}", serde_json::to_string(&booking).unwrap());

    // Submit booking and get available trains
    let trains: Vec<TrainInfo> = {
        // Submit booking info
        let url = format!("https://irs.thsrc.com.tw/IMINT/;jsessionid={}?wicket:interface=:0:BookingS1Form::IFormSubmitListener", session_id);
        println!("submit_booking_form_url: {}", url);
        let response = client.post(url)
            .headers(gen_common_headers())
            .form(&booking)
            .send()?;
        println!("submit booking response: {:?}", response);
        let response_text = response.text()?;
        println!("submit booking response text: {:?}", response_text);
        assert_submission_errors(response_text.clone())?;

        // Parse train info
        let document = Html::parse_document(&response_text);
        let trains = document
            .select(&Selector::parse("label").unwrap())
            .map(|label| {
                TrainInfo {
                    id: label.select(&Selector::parse("#QueryCode").unwrap()).next().unwrap().inner_html().parse().unwrap(),
                    depart: label.select(&Selector::parse("#QueryDeparture").unwrap()).next().unwrap().inner_html(),
                    arrive: label.select(&Selector::parse("#QueryArrival").unwrap()).next().unwrap().inner_html(),
                    travel_time: label.select(&Selector::parse(".duration > span:nth-of-type(2)").unwrap()).next().unwrap().inner_html(),
                    discount_str: parse_discount(label),
                    form_value: label.select(&Selector::parse(r#"input[name="TrainQueryDataViewPanel:TrainGroup"]"#).unwrap()).next().unwrap().value().attr("value").unwrap().to_string(),
                }
            })
            .collect();

        Ok::<Vec<TrainInfo>, ErrorMessages>(trains)
    }?;
    println!("trains: {:?}", trains);

    // Select train
    let train_selection = {
        for (idx, train) in trains.iter().enumerate() {
            println!("{item_num}. {train_id:>4} {train_depart:>3}~{train_arrive} {train_travel_time:>3} {train_discount_str}", item_num = idx + 1, train_id = train.id, train_depart = train.depart, train_arrive = train.arrive, train_travel_time = train.travel_time, train_discount_str = train.discount_str);
        }
        println!("Enter selection (default: 1): ");
        let mut train_selection_str = String::new();
        io::stdin().read_line(&mut train_selection_str)?;
        let trimmed_input = train_selection_str.trim();
        let train_selection = if trimmed_input.is_empty() {
            0
        } else {
            trimmed_input.parse::<usize>().unwrap() - 1
        };
        println!("Selected train: {train_selection}");

        TrainSelection {
            selected_train: trains[train_selection].form_value.clone(),
            form_mark: String::from(""),
        }
    };
    println!("train_selection: {:?}", train_selection);
    println!("train_selection (json): {}", serde_json::to_string(&train_selection).unwrap());

    // Submit train selection and prepare ticket info
    let ticket_confirmation = {
        // Submit train selection info
        let response = client.post("https://irs.thsrc.com.tw/IMINT/?wicket:interface=:1:BookingS2Form::IFormSubmitListener")
            .headers(gen_common_headers())
            .form(&train_selection)
            .send()?;
        println!("submit train selection response: {:?}", response);
        let response_text = response.text()?;
        println!("submit train selection response text: {:?}", response_text);
        assert_submission_errors(response_text.clone())?;

        // Prepare ticket info
        let document = Html::parse_document(&response_text);
        // TODO Get parameters either from presets or user input
        // TODO Fake the booking parameters for now
        Ok::<TicketConfirmation, ErrorMessages>(
            TicketConfirmation {
                personal_id: "".to_string(),
                phone_num: "".to_string(),
                member_radio: document
                    .select(&Selector::parse(r#"input[name="TicketMemberSystemInputPanel:TakerMemberSystemDataView:memberSystemRadioGroup"][checked]"#).unwrap())
                    .next().unwrap().value().attr("value").unwrap().to_string(),
                form_mark: "".to_string(),
                id_input_radio: 0,
                diff_over: 1,
                email: "".to_string(),
                agree: "on".to_string(),
                go_back_m: "".to_string(),
                back_home: "".to_string(),
                tgo_error: 1,
                elder_id0: "".to_string(),
                elder_id1: "".to_string(),
            }
        )
    }?;
    println!("ticket_confirmation: {:?}", ticket_confirmation);
    println!("ticket_confirmation (json): {}", serde_json::to_string(&ticket_confirmation).unwrap());

    Ok(())
}
