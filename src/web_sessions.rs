use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Sessions(HashMap<String, (Instant, String)>);
impl Sessions {
    pub fn create(&mut self, hash: &str) -> String {
        self.0
            .retain(|_, (t, _)| t.elapsed() < Duration::from_secs(28800));
        let bytes: [u8; 32] = rand::random();
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        self.0.insert(token.clone(), (Instant::now(), hash.into()));
        token
    }
    pub fn valid(&self, token: &str, hash: &str) -> bool {
        !hash.is_empty()
            && self
                .0
                .get(token)
                .is_some_and(|(t, h)| h == hash && t.elapsed() < Duration::from_secs(28800))
    }
    pub fn remove(&mut self, token: &str) {
        self.0.remove(token);
    }
}
pub fn cookie(value: &str) -> Option<&str> {
    value
        .split(';')
        .find_map(|v| v.trim().strip_prefix("tm_session="))
}
