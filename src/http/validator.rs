// use super::request::*;
use super::curl::*;
// use std::collections::HashMap;
// use std::cell::RefCell;
// use std::sync::{Mutex};

pub struct Validator {
    url: String,
    // cache: Mutex<HashMap<String, bool>>
    // cache: RefCell<HashMap<String, bool>>
}

impl Validator {
    pub fn new(url: String) -> Self {
        // Validator{url: url, cache: Mutex::new(HashMap::<String, bool>::new())}
        // Validator{url: url, cache: RefCell::new(HashMap::<String, bool>::new())}
        Validator { url: url }
    }
    pub fn CheckLimit(&self, chainpid: String) -> bool {
        // let mut buf = self.cache.lock().unwrap();
        // let mut buf = self.cache.borrow_mut();
        // let ok = buf.contains_key(&chainpid);
        // if ok {
        //     return true;
        // }

        // let client = HttpRequest::new();
        // block_on(client.GetSimple("https://elara.patract.io/stat/chain"));//test
        // let (ret, ok) = block_on(client.Get(&(self.url.clone()+&chainpid)));
        let client = RequestCurl;
        let (ret, ok) = client.Get(&(self.url.clone() + &chainpid));
        if !ok {
            return false;
        }
        // buf.insert(chainpid, true);
        return true;
    }
}
