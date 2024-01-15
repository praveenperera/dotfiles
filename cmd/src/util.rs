use rand::{distributions::Uniform, Rng};

pub fn random_ascii(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#$%^&*()_+{}|:<>?[];,./-=~";
    let mut rng = rand::thread_rng();
    let char_num = Uniform::from(0..CHARSET.len());

    (0..length)
        .map(|_| CHARSET[rng.sample(char_num)] as char)
        .collect()
}
