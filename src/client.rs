use crate::types::Country;

const BASE_URL: &str = "https://isdayoff.ru";

pub struct IsDayOff {
    base_url: String,
    country: Country,
    pre: bool, // учёт сокращённых дней
    sd: bool, // шестидневная рабочая неделя;
    covid: bool, // нерабочие дни в Covid-19
    holiday: bool, // учёт праздничных дней
    http: reqwest::Client,
}

impl IsDayOff {
    pub fn builder() -> IsDayOffBuilder {
        IsDayOffBuilder::new()
    }

    fn build_url(&self, year: i32, month: Option<u32>, day: Option<u32>) -> String {
        let mut url = reqwest::Url::parse(&self.base_url).expect("Invalid base URL")
            .join("/api/getdata").expect("invalid path");

        {
            let mut q = url.query_pairs_mut();
            q.append_pair("cc", self.country.as_str());
            q.append_pair("year", &year.to_string());
            if let Some(m) = month { q.append_pair("month", &format!("{m:02}")); }
            if let Some(d) = day { q.append_pair("day", &format!("{d:02}")); }
            if self.pre { q.append_pair("pre", "1"); }
            if self.sd { q.append_pair("sd", "1"); }
            if self.covid { q.append_pair("covid", "1"); }
            if self.holiday { q.append_pair("holiday", "1"); }
        }

        url.into()
    }
}

#[derive(Default)]
pub struct IsDayOffBuilder {
    base_url: Option<String>,
    country: Country,
    pre: bool,
    sd: bool,
    covid: bool,
    holiday: bool,
    http: reqwest::Client,
}

impl IsDayOffBuilder {
    pub fn new() -> Self {Self::default()}

    pub fn base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    pub fn country(mut self, country: Country) -> Self {
        self.country = country;
        self
    }

    pub fn pre(mut self, pre: bool) -> Self {
        self.pre = pre;
        self
    }

    pub fn sd(mut self, sd: bool) -> Self {
        self.sd = sd;
        self
    }

    pub fn covid(mut self, covid: bool) -> Self {
        self.covid = covid;
        self
    }

    pub fn holiday(mut self, holiday: bool) -> Self {
        self.holiday = holiday;
        self
    }

    pub fn http(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    pub fn build(self) -> IsDayOff {
        IsDayOff {
            base_url: self.base_url.unwrap_or_else(|| BASE_URL.into()),
            country: self.country,
            pre: self.pre,
            sd: self.sd,
            covid: self.covid,
            holiday: self.holiday,
            http: self.http,
        }
    }
}