macro_rules! constants {
    ($base_url:expr) => {
        pub const BASE_URL: &str = $base_url;
        pub const BOOKING_PAGE_URL: &str = concat!($base_url, "/IMINT/?locale=tw");
        pub const CAPTCHA_LOCAL_PATH: &str = "tmp/captcha.png";
        pub const SUBMIT_TRAIN_URL: &str = concat!($base_url, "/IMINT/?wicket:interface=:1:BookingS2Form::IFormSubmitListener");
        pub const SUBMIT_TICKET_CONFIRMATION_URL: &str = concat!($base_url, "/IMINT/?wicket:interface=:2:BookingS3Form::IFormSubmitListener");
    };
}

constants!("https://irs.thsrc.com.tw");
