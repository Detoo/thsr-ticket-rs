use std::error::Error;
use std::io::stdin;
use chrono::{Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HOST, USER_AGENT};
use scraper::{ElementRef, Html, Selector};
use strum::IntoEnumIterator;
use crate::BookingFormParams;
use crate::configs::BASE_URL;
use crate::models::{Booking, BookingPersisted, CabinClass, ErrorMessages, Preset, SeatPref, Station, Trip};

pub fn gen_booking_url(session_id: String) -> String {
    format!("{base_url}/IMINT/;jsessionid={session_id}?wicket:interface=:0:BookingS1Form::IFormSubmitListener", base_url=BASE_URL)
}

pub fn gen_common_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("irs.thsrc.com.tw"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.2311.135 Safari/537.36 Edge/12.246"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-TW,zh;q=0.8,en-US;q=0.5,en;q=0.3"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.clone()
}

pub fn gen_booking(booking_worksheet: &BookingPersisted, booking_form_params: &BookingFormParams, captcha_solution: String) -> Booking {
    Booking {
        persisted: booking_worksheet.clone(),
        search_by: booking_form_params.search_by_time_value.clone(),
        types_of_trip: Trip::OneWay, // We don't support round-trip
        security_code: captcha_solution,
        form_mark: String::from(""),
        inbound_date: None,
        inbound_time: None,
        to_train_id: None,
        back_train_id: None,
    }
}

pub fn ask_for_string() -> Result<String, Box<dyn Error>> {
    Ok(stdin().lines().next().unwrap()?.trim().to_string())
}

pub fn ask_for_station(leg_type: &str, default: Station) -> Result<Station, Box<dyn Error>> {
    // Print all options
    Station::iter().for_each(|station| {
        println!("({station_num}) {station_str}", station_num=station.clone() as u8, station_str=station.to_string());
    });

    println!("Select {leg_type} station (default: {}):", default.clone() as u8);
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(default.clone())
    } else {
        Ok(Station::from_repr(input_str.parse()?).unwrap())
    }
}

pub fn ask_for_date(leg_type: &str, tz: &Tz) -> Result<NaiveDate, Box<dyn Error>> {
    let today = Utc::now().with_timezone(tz).date_naive();
    let latest_date = today + Duration::days(30);

    println!("Select {leg_type} date ({today}~{latest_date}) (default: latest date):", today=format_date(today), latest_date=format_date(latest_date));
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(latest_date)
    } else {
        Ok(NaiveDate::parse_from_str(&*input_str, "%Y/%m/%d").unwrap())
    }
}

pub fn ask_for_time(leg_type: &str, booking_form_params: &BookingFormParams) -> Result<String, Box<dyn Error>> {
    // Print all options
    booking_form_params.time_options.iter().enumerate().for_each(|(idx, option)| {
        let parsed_option = if option.len() == 4 {
            // Ex. 930A
            format!("{}:{}", &option[..1], &option[1..])
        } else {
            // Ex. 1130A
            format!("{}:{}", &option[..2], &option[2..])
        };
        println!("({option_num}) {parsed_option}", option_num=idx + 1);
    });

    let default = 12;
    println!("Select {leg_type} time (default: {default}):");
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(booking_form_params.time_options[default - 1].clone())
    } else {
        Ok(booking_form_params.time_options[input_str.parse::<usize>()? - 1].clone())
    }
}

pub fn ask_for_seat(default: SeatPref) -> Result<SeatPref, Box<dyn Error>> {
    // Print all options
    SeatPref::iter().for_each(|seat_pref| {
        println!("({option_num}) {option_str}", option_num=seat_pref.clone() as u8, option_str=seat_pref.to_string());
    });

    println!("Select seat preference (default: {}):", default.clone() as u8);
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(default.clone())
    } else {
        Ok(SeatPref::from_repr(input_str.parse()?).unwrap())
    }
}

pub fn ask_for_class(default: CabinClass) -> Result<CabinClass, Box<dyn Error>> {
    // Print all options
    CabinClass::iter().for_each(|option| {
        println!("({option_num}) {option_str}", option_num=option.clone() as u8, option_str=option.to_string());
    });

    println!("Select cabin class (default: {}):", default.clone() as u8);
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(default.clone())
    } else {
        Ok(CabinClass::from_repr(input_str.parse()?).unwrap())
    }
}

pub fn ask_for_ticket_num(ticket_type: &str, ticket_descriptions: &str, default: u8) -> Result<String, Box<dyn Error>> {
    fn format_ticket_num(num: u8, ticket_type: &str) -> String {
        format!("{num}{ticket_type}")
    }

    println!("Select number of {ticket_descriptions} tickets (default: {default}):");
    let input_str = ask_for_string()?;
    if input_str.is_empty() {
        Ok(format_ticket_num(default, ticket_type))
    } else {
        Ok(format_ticket_num(input_str.parse()?, ticket_type))
    }
}

// TODO I tried to implement a generic `ask_for_enum<T>()` to reduce repetitive codes in `ask_for_seat()`, `ask_for_class()`, etc.;
//  however, `strum::FromRepr` derive does not provide a trait for building such generic functions.
//  It has been discussed in https://github.com/Peternator7/strum/issues/251 and there seems to be no solutions yet.
//
// ```rust
// pub fn ask_for_enum<T>(descriptions: &str, default: T) -> Result<T, Box<dyn Error>>
// where
//     T: IntoEnumIterator + FromRepr + Clone
// ```

pub fn format_date(d: NaiveDate) -> String {
    d.format("%Y/%m/%d").to_string()
}

pub fn parse_discount(item: ElementRef) -> String {
    let mut discounts: Vec<String> = Vec::new();
    if let Some(discount) = item.select(&Selector::parse("p.early-bird").unwrap()).next() {
        discounts.push(discount.inner_html());
    }
    if let Some(discount) = item.select(&Selector::parse("p.student").unwrap()).next() {
        discounts.push(discount.inner_html());
    }
    discounts.join(", ")
}

pub fn assert_submission_errors(response_text: String) -> Result<(), ErrorMessages> {
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

pub fn print_presets(presets: &Vec<Preset>) {
    presets.iter().enumerate().for_each(
        |(idx, preset)| {
            println!("Preset #{option_num}", option_num=idx + 1);
            println!("  Personal ID:         {}", preset.ticket_confirmation.personal_id);
            println!("  Phone:               {}", preset.ticket_confirmation.phone_num);
            println!("  Depart Station:      {:?}", preset.booking.start_station);
            println!("  Destination Station: {:?}", preset.booking.dest_station);
            println!("  Depart Date:         {}", preset.booking.outbound_date);
            println!("  Depart Time:         {}", preset.booking.outbound_time);
            println!("  Cabin Class:         {:?}", preset.booking.class_type);
            println!("  Seat Preference:     {:?}", preset.booking.seat_prefer);
            // TODO This is hard-coded for now
            println!("  Adult ticket number: {}", preset.booking.adult_ticket_num);
            println!("  Elder ticket number: {}", preset.booking.elder_ticket_num);
            println!("  Elder ID #1:         {}", preset.ticket_confirmation.elder_id0);
            println!("  Elder ID #2:         {}", preset.ticket_confirmation.elder_id1);
            println!();
        }
    );
}