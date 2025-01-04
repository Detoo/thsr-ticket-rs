use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{EnumIter, FromRepr, Display};

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr, EnumIter, FromRepr, Display, Clone)]
#[repr(u8)]
pub enum Station {
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
pub enum Trip {
    OneWay = 0,
    RoundTrip,
}

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr, EnumIter, FromRepr, Display, Default, Clone)]
#[repr(u8)]
pub enum CabinClass {
    #[default]
    Standard = 0,
    Business,
}

#[derive(Debug, PartialEq, Serialize_repr, Deserialize_repr, EnumIter, FromRepr, Display, Clone)]
#[repr(u8)]
pub enum SeatPref {
    NoPref = 0,
    Window,
    Aisle,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BookingPersisted {
    #[serde(rename = "selectStartStation")]
    pub start_station: Station,
    #[serde(rename = "selectDestinationStation")]
    pub dest_station: Station,
    #[serde(rename = "toTimeInputField")]
    pub outbound_date: String,
    #[serde(rename = "toTimeTable")]
    pub outbound_time: String,
    #[serde(rename = "seatCon:seatRadioGroup")]
    pub seat_prefer: SeatPref,
    #[serde(default, rename = "trainCon:trainRadioGroup")]
    pub class_type: CabinClass,

    // TODO There must be a better way to represent this
    #[serde(default = "default_adult_ticket", rename = "ticketPanel:rows:0:ticketAmount")]
    pub adult_ticket_num: String,
    #[serde(default = "default_child_ticket", rename = "ticketPanel:rows:1:ticketAmount")]
    pub child_ticket_num: String,
    #[serde(default = "default_disabled_ticket", rename = "ticketPanel:rows:2:ticketAmount")]
    pub disabled_ticket_num: String,
    #[serde(default = "default_elder_ticket", rename = "ticketPanel:rows:3:ticketAmount")]
    pub elder_ticket_num: String,
    #[serde(default = "default_college_ticket", rename = "ticketPanel:rows:4:ticketAmount")]
    pub college_ticket_num: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Booking {
    #[serde(flatten)]
    pub persisted: BookingPersisted,

    #[serde(rename = "bookingMethod")]
    pub search_by: String,
    #[serde(rename = "tripCon:typesoftrip")]
    pub types_of_trip: Trip,
    #[serde(rename = "homeCaptcha:securityCode")]
    pub security_code: String,
    #[serde(default, rename = "BookingS1Form:hf:0")]
    pub form_mark: String,
    #[serde(default, rename = "backTimeInputField")]
    pub inbound_date: Option<String>,
    #[serde(default, rename = "backTimeTable")]
    pub inbound_time: Option<String>,
    #[serde(default, rename = "toTrainIDInputField")]
    pub to_train_id: Option<i16>,
    #[serde(default, rename = "backTrainIDInputField")]
    pub back_train_id: Option<i16>,
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
pub struct TrainInfo {
    pub id: i16,
    pub depart: String,
    pub arrive: String,
    pub travel_time: String,
    pub discount_str: String,
    pub form_value: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainSelection {
    #[serde(rename = "TrainQueryDataViewPanel:TrainGroup")]
    pub selected_train: String,
    #[serde(default, rename = "BookingS2Form:hf:0")]
    pub form_mark: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TicketConfirmationPersisted {
    #[serde(rename = "dummyId")]
    pub personal_id: String,
    #[serde(rename = "dummyPhone")]
    pub phone_num: String,

    // TODO Make it dynamic. Current implementation assumes 1 adult, 2 elder because the aliases are type and order dependent
    #[serde(default, rename = "TicketPassengerInfoInputPanel:passengerDataView:1:passengerDataView2:passengerDataIdNumber")]
    pub elder_id0: String,
    #[serde(default, rename = "TicketPassengerInfoInputPanel:passengerDataView:2:passengerDataView2:passengerDataIdNumber")]
    pub elder_id1: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TicketConfirmation {
    #[serde(flatten)]
    pub persisted: TicketConfirmationPersisted,
    #[serde(rename = "TicketMemberSystemInputPanel:TakerMemberSystemDataView:memberSystemRadioGroup")]
    pub member_radio: String,
    #[serde(default, rename = "BookingS3FormSP:hf:0")]
    pub form_mark: String,
    #[serde(default, rename = "idInputRadio")]
    pub id_input_radio: i8,
    #[serde(default = "default_1_i8", rename = "diffOver")]
    pub diff_over: i8,
    #[serde(default, rename = "email")]
    pub email: String,
    #[serde(default = "default_agree", rename = "agree")]
    pub agree: String,
    #[serde(default, rename = "isGoBackM")]
    pub go_back_m: String,
    #[serde(default, rename = "backHome")]
    pub back_home: String,
    #[serde(default = "default_1_i8", rename = "TgoError")]
    pub tgo_error: i8,
}

fn default_1_i8() -> i8 {
    1
}

fn default_agree() -> String {
    "on".to_string()
}

// TODO Dynamically disable renaming in Preset serialization
//  We don't want the field names being renamed when saving as a preset because it actually hurts readability.
//  Unfortunately serde currently does not support dynamic disabling renaming,
//  so we are stuck with it for now.
//  Note: in case it wasn't clear, we can't get rid of the renaming either because we must submit the form using those names.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub booking: BookingPersisted,
    pub ticket_confirmation: TicketConfirmationPersisted,
}

pub struct BookingFormParams {
    pub session_id: String,
    pub search_by_time_value: String,
    pub time_options: Vec<String>,
}

pub struct TicketConfirmationFormParams {
    pub member_value: String,
}

#[derive(Debug)]
pub struct ErrorMessages {
    pub errors: Vec<String>,
}

impl fmt::Display for ErrorMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.errors)
    }
}

impl Error for ErrorMessages {}
