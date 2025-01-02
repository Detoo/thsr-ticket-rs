mod configs;
mod models;
mod utils;

use crate::models::{Booking, CabinClass, SeatPref, Station, TicketConfirmation, TrainInfo, TrainSelection, Trip};
use crate::utils::{assert_submission_errors, gen_booking_url, gen_common_headers, parse_discount};
use opener;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use scraper::{Element, Html, Selector};
use std::error::Error;
use std::path::Path;
use std::{fs::File, io::{self, Write}};
use log::{debug, LevelFilter};

struct App {
    client: Client,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client: Client::builder()
                .redirect(Policy::default())
                .cookie_store(true)
                .build()?,
        })
    }

    fn start_session_with_captcha(&mut self) -> Result<String, Box<dyn Error>> {
        let response = self.client
            .get(configs::BOOKING_PAGE_URL)
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
        let captcha_url = [configs::BASE_URL, src].concat();
        // Download and open image
        let response = self.client.get(captcha_url).headers(gen_common_headers()).send()?;
        let bytes = response.bytes()?;
        let path = Path::new(configs::CAPTCHA_LOCAL_PATH);
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        opener::open(path)?;

        Ok(session_id)
    }

    fn solve_captcha(&mut self) -> Result<String, Box<dyn Error>> {
        println!("Type the answer to the CAPTCHA: ");
        let mut captcha_solution = String::new();
        io::stdin().read_line(&mut captcha_solution)?;

        Ok(captcha_solution.trim().to_string())
    }

    fn prepare_booking(&mut self, captcha_solution: String) -> Result<Booking, Box<dyn Error>> {
        // TODO Get booking parameters either from presets or user input
        // TODO Fake the booking parameters for now

        // TODO test
        // let dt = NaiveDateTime::parse_from_str("2025/01/27 22:00", "%Y/%m/%d %H:%M").unwrap();
        // println!("datetime: {}", dt);

        // TODO Test booking
        Ok(Booking{
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
            security_code: captcha_solution,
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
        })
    }

    fn submit_booking_and_get_trains(&self, session_id: String, booking: Booking) -> Result<Vec<TrainInfo>, Box<dyn Error>> {
        // Submit booking info
        let url = gen_booking_url(session_id);
        debug!("submit_booking_form_url: {}", url);
        let response = self.client.post(url)
            .headers(gen_common_headers())
            .form(&booking)
            .send()?;
        debug!("submit booking response: {:?}", response);
        let response_text = response.text()?;
        debug!("submit booking response text: {:?}", response_text);
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

        Ok(trains)
    }

    fn select_train(&self, trains: Vec<TrainInfo>) -> Result<TrainSelection, Box<dyn Error>> {
        println!("Option  Train   Depart  Arrive  Duration  Discount");
        for (idx, train) in trains.iter().enumerate() {
            println!("{item_str:<8}{train_id:<8}{train_depart:<8}{train_arrive:<8}{train_travel_time:<10}{train_discount_str}", item_str = format!("({})", idx + 1), train_id = train.id, train_depart = train.depart, train_arrive = train.arrive, train_travel_time = train.travel_time, train_discount_str = train.discount_str);
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
        debug!("Selected option: {}", train_selection + 1);

        Ok(TrainSelection {
            selected_train: trains[train_selection].form_value.clone(),
            form_mark: String::from(""),
        })
    }

    fn submit_train_selection(&self, train_selection: TrainSelection) -> Result<TicketConfirmation, Box<dyn Error>> {
        // Submit train selection info
        let response = self.client.post(configs::SUBMIT_TRAIN_URL)
            .headers(gen_common_headers())
            .form(&train_selection)
            .send()?;
        debug!("submit train selection response: {:?}", response);
        let response_text = response.text()?;
        debug!("submit train selection response text: {:?}", response_text);
        assert_submission_errors(response_text.clone())?;

        // Prepare ticket info
        let document = Html::parse_document(&response_text);
        // TODO Get parameters either from presets or user input
        // TODO Fake the booking parameters for now
        Ok(
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
    }

    fn submit_ticket_confirmation(&self, ticket_confirmation: TicketConfirmation) -> Result<(), Box<dyn Error>> {
        // Submit ticket confirmation
        let response = self.client.post(configs::SUBMIT_TICKET_CONFIRMATION_URL)
            .headers(gen_common_headers())
            .form(&ticket_confirmation)
            .send()?;
        debug!("submit ticket confirmation response: {:?}", response);
        let response_text = response.text()?;
        debug!("submit ticket confirmation response text: {:?}", response_text);
        assert_submission_errors(response_text.clone())?;

        // Parse ticket
        let document = Html::parse_document(&response_text);
        println!("\n\n----------- Booking Results -----------");
        println!("Ticket ID: {}", document.select(&Selector::parse("p.pnr-code > span:first-child").unwrap()).next().unwrap().inner_html());
        println!("Total price: {}", document.select(&Selector::parse("#setTrainTotalPriceValue").unwrap()).next().unwrap().inner_html());
        println!("---------------------------------------");
        println!("Date    From   Dest    Depart  Arrive  Train");
        println!(
            "{:<8}{:<6}{:<6}{:<8}{:<8}{:<8}",
            document.select(&Selector::parse("span.date > span").unwrap()).next().unwrap().inner_html(),
            document.select(&Selector::parse("p.departure-stn > span").unwrap()).next().unwrap().inner_html(),
            document.select(&Selector::parse("p.arrival-stn > span").unwrap()).next().unwrap().inner_html(),
            document.select(&Selector::parse("#setTrainDeparture0").unwrap()).next().unwrap().inner_html(),
            document.select(&Selector::parse("#setTrainArrival0").unwrap()).next().unwrap().inner_html(),
            document.select(&Selector::parse("#setTrainCode0").unwrap()).next().unwrap().inner_html(),
        );
        let seat_class = document.select(&Selector::parse("p.info-title").unwrap())
            .find(|elem| { elem.inner_html() == "車廂" }).unwrap()
            .next_sibling_element().unwrap()
            .select(&Selector::parse("span").unwrap())
            .next().unwrap().inner_html();
        document.select(&Selector::parse("div.seat-label > span").unwrap())
            .for_each(|elem| {
                println!("{seat_class} {}", elem.inner_html());
            });

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    log::set_max_level(LevelFilter::Info);

    // Start a new session
    let mut app = App::new()?;
    let session_id = app.start_session_with_captcha()?;
    debug!("JSESSIONID: {}", session_id);

    // Get user input for CAPTCHA
    let captcha_solution = app.solve_captcha()?;
    debug!("CAPTCHA solution entered: {}", captcha_solution);

    // Prepare booking info
    let booking = app.prepare_booking(captcha_solution)?;
    debug!("booking: {:?}", booking);
    debug!("booking (json): {}", serde_json::to_string(&booking).unwrap());

    // Submit booking and get available trains
    let trains = app.submit_booking_and_get_trains(session_id, booking)?;
    debug!("trains: {:?}", trains);

    // Select train
    let train_selection = app.select_train(trains)?;
    debug!("train_selection: {:?}", train_selection);
    debug!("train_selection (json): {}", serde_json::to_string(&train_selection).unwrap());

    // Submit train selection and prepare ticket info
    let ticket_confirmation = app.submit_train_selection(train_selection)?;
    debug!("ticket_confirmation: {:?}", ticket_confirmation);
    debug!("ticket_confirmation (json): {}", serde_json::to_string(&ticket_confirmation).unwrap());

    // Submit train selection and prepare ticket info
    app.submit_ticket_confirmation(ticket_confirmation)?;

    Ok(())
}
