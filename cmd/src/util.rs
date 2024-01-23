use rand::{
    distributions::{Alphanumeric, DistString, Uniform},
    Rng,
};

pub fn random_ascii(length: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_pin(length: usize) -> String {
    const CHARSET: &[u8] = b"01234567890";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}

pub fn random_alpha_numeric(length: usize) -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), length)
}
