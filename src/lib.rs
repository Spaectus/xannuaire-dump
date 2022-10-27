extern crate reqwest;
extern crate base64;

use std::cmp::max;
use std::collections::HashMap;
use std::time;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use reqwest::blocking::Client as BlockingClient;
use reqwest::header;

use scraper::{ElementRef, Html};
use scraper::Selector;

use rand_distr::{Distribution};

use csv::Writer;

use serde::Serialize;

const NO_PHOTO_HASH: u64 = 12223883342556761564;

const LOGIN_URL: &str = "https://extranet.polytechnique.fr/xannuaire/login/index.php";
const AUTH_URL: &str = "https://extranet.polytechnique.fr/xannuaire/login/switch.php";
const SEARCH_PERSON_URL: &str = "https://extranet.polytechnique.fr/xannuaire/search/searchpersonne.php";

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn random_wait() {
    let mut rng = rand::thread_rng();
    let normal = rand_distr::Normal::new(1500., 7500.).unwrap();
    let random_ = normal.sample(&mut rng).round() as i64;
    let time_millis: u64 = max(random_, 800) as u64;
    println!("Waiting {} millis seconds", time_millis);
    std::thread::sleep(time::Duration::from_millis(time_millis));
}

#[derive(Serialize)]
struct FullPerson {
    uid: String,
    name: String,
    #[serde(rename = "rattach")]
    rattach: String,
    #[serde(rename = "rattach_full")]
    rattach_full: String,
    phone_number: String,
    email: String,
    desk: String,
    image_xid: String,
    image_base64: String,
}

#[derive(Serialize)]
struct Person {
    uid: String,
    name: String,
    #[serde(rename = "rattach")]
    rattach: String,
    #[serde(rename = "rattach_full")]
    rattach_full: String,
    phone_number: String,
}

fn build_person_from_ul(ul_element: &ElementRef) -> Person {
    let error_msg = "This software is obsolete";

    let selector_a = Selector::parse("li>a").unwrap();
    let a = ul_element.select(&selector_a).next().expect(error_msg);
    let name = a.inner_html();
    let uid: String = a.value().attr("href").expect(error_msg)
        .split("uid=").nth(1).unwrap().to_string();

    let selector_second_li = Selector::parse("li:nth-child(2)").unwrap();
    let second_li = ul_element.select(&selector_second_li).next().expect(error_msg);
    let rattach = second_li.inner_html();

    let selector_thrid_li = Selector::parse("li:nth-child(3)").unwrap();
    let third_li = ul_element.select(&selector_thrid_li).next().expect(error_msg);
    let rattach_full = third_li.inner_html();

    let o_fourth_li = ul_element.select(
        &Selector::parse("li:nth-child(4)").unwrap()
    ).next();
    let phone_number: String =
        if let Some(fourth_li) = o_fourth_li {
            fourth_li.inner_html()
        } else {
            "".to_string()
        };


    Person {
        uid: uid,
        rattach: rattach,
        rattach_full: rattach_full,
        name: name,
        phone_number: phone_number
    }
}

struct Page {
    client: BlockingClient,
    token: String,
}

fn next_sibling(e: ElementRef) -> Option<ElementRef> {
    let a = e.next_siblings().find(|s| s.value().is_element());
    if a.is_none() {
        return None;
    }
    ElementRef::wrap(a.unwrap())
}

impl Page {
    fn req(&mut self, rattach: String) -> Vec<Person> {
        let res = self.client.post(SEARCH_PERSON_URL)
            .body(format!("nom=&prenom=&emploi=&telephone=&rattach={}&token={}", &rattach, &self.token))
            .send().unwrap();

        println!("Scraping for rattach \"{}\" with token : {}, status : {}", &rattach, &self.token, res.status());
        let text = &res.text().unwrap();
        let document = Html::parse_document(text);
        let selector = Selector::parse("div.web div.row ul.liste_resultats").unwrap();
        let ul_elements = document.select(&selector);

        let persons: Vec<Person> = ul_elements.map(
            |ul_ele| build_person_from_ul(&ul_ele)
        ).collect();

        self.token = extract_token(&document);


        persons
    }

    fn complete_person(&mut self, person: Person) -> FullPerson {
        let res = self.client.get(format!("https://extranet.polytechnique.fr/xannuaire/search/index.php?uid={}", &person.uid))
            .send().unwrap();

        let document = Html::parse_document(&res.text().unwrap());

        let selector_blocinfoprincipale_first_dt = Selector::parse("#blockFichePerso div.blocinfoPrincipale dl>dt:nth-child(1)").unwrap();
        let blocinfoprincipale_first_dt = document.select(&selector_blocinfoprincipale_first_dt).next().unwrap();

        let mut current = blocinfoprincipale_first_dt;
        let mut current_dd = next_sibling(blocinfoprincipale_first_dt).unwrap();

        let mut desk = "".to_string();
        let mut email = "".to_string();
        let mut phone_number = person.phone_number.clone();
        let mut image_base64 = "".to_string();

        loop {
            let dt_text = current.inner_html();

            if dt_text.contains("Bureau") {
                desk = current_dd.inner_html();
            } else if dt_text.contains("Courriel") {
                email = current_dd.select(&Selector::parse("a").unwrap()).next().unwrap().inner_html();
            } else if dt_text.contains("Téléphone") {
                phone_number = current_dd.select(&Selector::parse("a").unwrap()).next().unwrap().inner_html()
            } else if dt_text == "Structures(s) de rattachement : " {
                // do nothing
            } else {
                println!("Information \"{}\" is not recorded by this software (see in {} profil for example)", dt_text, &person.uid);
            }

            if let Some(j) = next_sibling(current_dd) {
                current = j;
                current_dd = next_sibling(j).unwrap();
            } else {
                break;
            }
        }

        let img_element = document.select(
            &Selector::parse("#blockFichePerso #photo").unwrap()
        ).next().unwrap();
        let xid = img_element.value().attr("src").unwrap()
            .split_once("?xid=").unwrap().1.to_string();

        let o_res = self.client.get(format!("https://extranet.polytechnique.fr/xannuaire/search/imageunique.php?xid={}", &xid)).send();
        if let Ok(res) = o_res {
            let base64 = base64::encode(res.bytes().unwrap());
            let hash = calculate_hash(&base64);
            //println!("hash of {} photo : {}", hash, person.name);
            if hash != NO_PHOTO_HASH {
                image_base64 = base64;
            }
        } else {
            println!("Can't get the photo of {} (xid = {})", person.name, xid);
        }

        FullPerson {
            uid: person.uid,
            name: person.name,
            rattach: person.rattach,
            rattach_full: person.rattach_full,
            phone_number: phone_number,
            email: email,
            desk: desk,
            image_xid: xid,
            image_base64: image_base64,
        }
    }
}

fn get_structures(document: &Html) -> HashMap<String, String> {
    let selector = Selector::parse("#validerForm ul.dropdown-menu>li>a").unwrap();

    let structures: HashMap<String, String> = document.select(&selector).map(|a_element| {
        (a_element.value().attr("id").unwrap().to_string(), a_element.inner_html())
    }).collect();

    structures
}

fn extract_token(document: &Html) -> String {
    let selector = Selector::parse("#token").unwrap();
    let hidden_input = document.select(&selector).next().expect("Can't find token");
    let token = hidden_input.value().attr("value").unwrap();
    token.to_string()
}

fn get_auth_token(client: &BlockingClient) -> (String, String) {
    let res = client.get(LOGIN_URL)
        .send().expect("Connection error. Are you connected to Internet?");

    //println!("header : {}", res.headers().get("set-cookie").unwrap().to_str().unwrap());

    let phpsessid_token = res.headers().get("set-cookie").unwrap().to_str().unwrap() //PHPSESSID=rez54013c573f872e9ece2ca; expires=Thu, 31-Oct-2018 19:33:59 GMT; Max-Age=7200; path=/; HttpOnly
        .split_once("PHPSESSID=").unwrap().1.split(";").nth(0).unwrap();
    let cook = format!("PHPSESSID={}", phpsessid_token);

    let res_html_str = res.text().unwrap();
    let document = Html::parse_document(&res_html_str);
    let selector = Selector::parse("#tokenXAnnuaire").unwrap();
    let hidden_input = document.select(&selector).next().expect("Can't find auth token");
    let token = hidden_input.value().attr("value").unwrap();
    (token.to_string(), cook)
}

fn auth(client: &BlockingClient, username: &str, password: &str, token: &str) -> Html {
    let req = client.post(AUTH_URL)
        .body(format!("login={}&password={}&tokenXAnnuaire={}", username, password, token));

    let res = req.send().unwrap();
    let url = res.url().clone();

    let document = Html::parse_document(&res.text().unwrap());
    let err = url.query_pairs().find(|(key, _)| key == "err").clone();

    if let Some((_, num_err)) = err {
        let mut msg_err = "Error when extracting the error message from the login page";
        let o_div_element = document.select(
            &Selector::parse("div.container div.alert").unwrap()
        ).next();
        if let Some(div_element) = o_div_element {
            msg_err = div_element.text().nth(1).unwrap();
        }
        panic!("Error of type of {} during authentication : \"{}\"", num_err, msg_err);
    }

    document
}

pub fn main(username: &str, password: &str, brief_mode: bool, slow_mode: bool, path: &std::path::PathBuf) {
    let mut headers = header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:106.0) Gecko/20100101 Firefox/106.0".parse().unwrap());
    headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse().unwrap());
    headers.insert("Accept-Language", "fr,fr-FR;q=0.8,en-US;q=0.5,en;q=0.3".parse().unwrap());
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Upgrade-Insecure-Requests", "1".parse().unwrap());
    headers.insert("Sec-Fetch-Dest", "document".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "navigate".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "none".parse().unwrap());
    headers.insert("Sec-Fetch-User", "?1".parse().unwrap());
    headers.insert("DNT", "1".parse().unwrap());
    headers.insert("Sec-GPC", "1".parse().unwrap());

    let client = reqwest::blocking::Client::builder()
        //.redirect(reqwest::redirect::Policy::none())
        .default_headers(headers.clone())
        .cookie_store(true)
        .build()
        .unwrap();


    let (token, phpsessid) = get_auth_token(&client);
    //println!("head : {}", );
    //println!("AUTH token {}", token);
    println!("{}", phpsessid);

    headers.insert("Content-Type", "application/x-www-form-urlencoded".parse().unwrap());
    headers.insert("Origin", "https://extranet.polytechnique.fr".parse().unwrap());
    headers.insert("Referer", "https://extranet.polytechnique.fr/xannuaire/login/index.php".parse().unwrap());
    headers.insert(header::COOKIE, phpsessid.parse().unwrap());
    headers.insert("Sec-Fetch-Site", "same-origin".parse().unwrap());

    let auth_client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .build()
        .unwrap();

    if slow_mode {
        random_wait();
    }

    let document = auth(&auth_client, username, password, &token);

    let structures = get_structures(&document);
    let current_token = extract_token(&document);

    let mut page = Page {
        client: auth_client,
        token: current_token,
    };

    let mut wtr = Writer::from_path(path).unwrap();

    for (struct_id, _struct_name) in &structures {
        if slow_mode {
            random_wait();
        }
        let vec_person = page.req(struct_id.to_string());

        for person in vec_person {
            //println!("{}", person.name);

            if brief_mode {
                wtr.serialize(person).unwrap();
            } else {
                if slow_mode {
                    random_wait();
                }
                wtr.serialize(page.complete_person(person)).unwrap();
            }

        }
    }
}

