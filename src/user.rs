#[repr(i32)]
enum Level {
    Default = 100,
    Trusted = 500,
    ChanTrusted = 8999,
    ChanMod = 9999,
    ChanOwner = 99999,
    Mod = 999999,
    Admin = 9999999,
}

pub struct User {
    pub username: String,
    pub level: i32,
}

impl User {
    pub fn has_level(&self, level: Level) -> bool {
        self.level >= level as i32
    }
}
