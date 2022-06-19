const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ
                            abcdefghijklmnopqrstuvwxyz
                            0123456789)(*&^%$#@!~";

use crate::constants::AVATARS;
use rand::Rng;

pub fn random_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn random_avatar() -> String {
    let mut rng = rand::thread_rng();
    AVATARS[rng.gen_range(0..AVATARS.len())].to_string()
}
