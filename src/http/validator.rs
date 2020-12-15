use super::curl::*;


#[derive(Clone)]
pub struct Validator {
    url: String,
}

impl Validator {
    pub fn new(url: String) -> Self {
        Validator{url: url}
    }
    pub fn CheckLimit(&self, chainpid: String) -> bool {
        let client = RequestCurl;
        let (ret, ok) = client.Get(&(self.url.clone()+&chainpid));
        if !ok {
            return false;
        }
        return true;    
        
    }
}
