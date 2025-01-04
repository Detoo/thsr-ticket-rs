mod configs;
mod models;
mod utils;

use crate::models::{Booking, BookingFormParams, BookingPersisted, CabinClass, Preset, SeatPref, Station, TicketConfirmation, TicketConfirmationFormParams, TicketConfirmationPersisted, TrainInfo, TrainSelection};
use crate::utils::{ask_for_class, ask_for_date, ask_for_string_with_descriptions, ask_for_seat, ask_for_station, ask_for_ticket_num, ask_for_time, assert_submission_errors, format_date, gen_booking, gen_booking_url, gen_common_headers, gen_ticket_confirmation, parse_discount, print_presets, print_preset};
use opener;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use scraper::{Element, Html, Selector};
use std::error::Error;
use std::path::Path;
use std::{fs::File, io::{self, Write}};
use std::io::BufReader;
use chrono_tz::Tz;
use chrono_tz::Tz::Asia__Taipei;
use clap::{arg, Parser};
use log::debug;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Auto-select preset #
    #[arg(short, long)]
    preset: Option<usize>,
}

#[derive(Debug)]
struct App {
    args: Args,
    client: Client,
    tz: Tz,
    booking_worksheet: Option<BookingPersisted>,
    ticket_confirmation_worksheet: Option<TicketConfirmationPersisted>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            args: Args::parse(),
            client: Client::builder()
                .redirect(Policy::default())
                .cookie_store(true)
                .build()?,
            tz: Asia__Taipei,
            booking_worksheet: None,
            ticket_confirmation_worksheet: None,
        })
    }

    fn prepare_preset(&mut self) -> Result<(), Box<dyn Error>> {
        // Load presets
        let presets = match File::open(configs::PRESETS_PATH) {
            Ok(file) => Ok(serde_json::from_reader::<_, Vec<Preset>>(BufReader::new(file))?),
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    println!("Presets not found in {}, skip", configs::PRESETS_PATH);
                    Ok(Vec::new())
                },
                _ => Err(err)
            },
        }?;

        match self.args.preset {
            Some(preset_num) => {
                // Load the preset if specified
                println!("Auto-select preset:");
                let preset = &presets[preset_num - 1];
                print_preset(preset_num, preset);
                self.load_preset(preset);
            },
            None => {
                // Otherwise, only ask user for it if available
                if presets.len() > 0 {
                    // Ask for preset selection
                    print_presets(&presets);

                    println!("Select the preset to load (default: ask for new info):");
                    let mut preset_idx_str = String::new();
                    io::stdin().read_line(&mut preset_idx_str)?;
                    let preset_idx_str_trimmed = preset_idx_str.trim().to_string();

                    // If user selected a preset
                    if preset_idx_str_trimmed.len() > 0 {
                        self.load_preset(&presets[preset_idx_str_trimmed.parse::<usize>()? - 1]);
                    }
                }
            }
        };

        Ok(())
    }

    fn load_preset(&mut self, preset: &Preset) {
        self.booking_worksheet = Some(preset.booking.clone());
        self.ticket_confirmation_worksheet = Some(preset.ticket_confirmation.clone());
    }

    fn start_session_with_captcha(&mut self) -> Result<BookingFormParams, Box<dyn Error>> {
        let response = self.client
            .get(configs::BOOKING_PAGE_URL)
            .headers(gen_common_headers())
            .send()?;

        // Find session ID
        let session_id = response.cookies().find(|cookie| cookie.name() == "JSESSIONID").unwrap().value().to_string();

        let response_text = response.text()?;
        let document = Html::parse_document(&response_text);

        // Find all essential parameters
        let search_by_time_value = document.select(&Selector::parse(r#"input[name="bookingMethod"][data-target="search-by-time"]"#).unwrap()).next().unwrap().value().attr("value").unwrap().to_string();
        debug!("search-by-time parameter: {search_by_time_value}");
        let time_options: Vec<String> = document
            .select(&Selector::parse(r#"select[name="toTimeTable"] > option:not([selected])"#).unwrap())
            .map(|elem| {
                elem.value().attr("value").unwrap().to_string()
            })
            .collect();
        debug!("time_options: {:?}", time_options);

        // Show CAPTCHA image
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

        Ok(BookingFormParams{
            session_id,
            search_by_time_value,
            time_options,
        })
    }

    fn solve_captcha(&mut self) -> Result<String, Box<dyn Error>> {
        println!("Type the answer to the CAPTCHA: ");
        let mut captcha_solution = String::new();
        io::stdin().read_line(&mut captcha_solution)?;

        Ok(captcha_solution.trim().to_string())
    }

    fn prepare_booking(&mut self, booking_form_params: &BookingFormParams, captcha_solution: String) -> Result<Booking, Box<dyn Error>> {
        match &self.booking_worksheet {
            // Preset exists
            Some(booking_worksheet) => Ok(gen_booking(
                booking_worksheet,
                booking_form_params,
                captcha_solution,
            )),
            // No preset, ask the user for more info
            None => Ok(gen_booking(
                &BookingPersisted {
                    start_station: ask_for_station("departure", Station::Nangang)?,
                    dest_station: ask_for_station("destination", Station::Zuouing)?,
                    outbound_date: format_date(ask_for_date("departure", &self.tz)?),
                    outbound_time: ask_for_time("departure", booking_form_params)?,
                    seat_prefer: ask_for_seat(SeatPref::NoPref)?,
                    class_type: ask_for_class(CabinClass::Standard)?,
                    adult_ticket_num: ask_for_ticket_num("F", "adult", 1)?,
                    elder_ticket_num: ask_for_ticket_num("E", "elder", 0)?,

                    // TODO Currently not supported
                    child_ticket_num: "0H".to_string(),
                    disabled_ticket_num: "0W".to_string(),
                    college_ticket_num: "0F".to_string(),
                },
                booking_form_params,
                captcha_solution,
            )),
        }
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

        let document = Html::parse_document(&response_text);
        let ticket_confirmation_form_params = TicketConfirmationFormParams {
            member_value: document
                .select(&Selector::parse(r#"input[name="TicketMemberSystemInputPanel:TakerMemberSystemDataView:memberSystemRadioGroup"][checked]"#).unwrap())
                .next().unwrap().value().attr("value").unwrap().to_string(),
        };

        match &self.ticket_confirmation_worksheet {
            // Preset exists
            Some(ticket_confirmation_worksheet) => Ok(gen_ticket_confirmation(
                ticket_confirmation_worksheet,
                &ticket_confirmation_form_params,
            )),
            // No preset, ask the user for more info
            None => Ok(gen_ticket_confirmation(
                &TicketConfirmationPersisted {
                    personal_id: ask_for_string_with_descriptions("personal ID")?,
                    phone_num: ask_for_string_with_descriptions("phone number")?,
                    elder_id0: ask_for_string_with_descriptions("elderly personal ID #1")?,
                    elder_id1: ask_for_string_with_descriptions("elderly personal ID #2")?,
                },
                &ticket_confirmation_form_params,
            ))
        }
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
    // Control logging level through env var `RUST_LOG`
    env_logger::init();

    // Start a new session
    let mut app = App::new()?;
    debug!("app inited: {:?}", app);

    app.prepare_preset()?;

    let booking_form_params = app.start_session_with_captcha()?;
    debug!("JSESSIONID: {}", booking_form_params.session_id);

    // Get user input for CAPTCHA
    let captcha_solution = app.solve_captcha()?;
    debug!("CAPTCHA solution entered: {}", captcha_solution);

    // Prepare booking info
    let booking = app.prepare_booking(&booking_form_params, captcha_solution)?;
    debug!("booking: {:?}", booking);
    debug!("booking (json): {}", serde_json::to_string(&booking).unwrap());

    // Submit booking and get available trains
    let trains = app.submit_booking_and_get_trains(booking_form_params.session_id, booking)?;
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
