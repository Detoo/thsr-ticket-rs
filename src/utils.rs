use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HOST, USER_AGENT};
use scraper::{ElementRef, Html, Selector};
use crate::configs::BASE_URL;
use crate::models::ErrorMessages;

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
