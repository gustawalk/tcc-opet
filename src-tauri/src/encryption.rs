use once_cell::sync::Lazy;

pub const ACTIVE_KEY_VERSION: u8 = 1;

static MASTER_KEY: Lazy<[u8; 32]> = Lazy::new(|| {
    let value = env!("OPETS_DATA_KEY_V1");
    let mut key = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let hex = std::str::from_utf8(chunk).expect("build key must be UTF-8 hex");
        key[index] = u8::from_str_radix(hex, 16).expect("build key must be valid hex");
    }
    key
});

pub fn derive_key(context: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&*MASTER_KEY);
    *hasher.finalize().as_bytes()
}

pub fn database_key() -> [u8; 32] {
    derive_key("com.walk.tcc-opet/database/v1")
}

pub fn attachment_key() -> [u8; 32] {
    derive_key("com.walk.tcc-opet/attachments/v1")
}

pub fn metadata_authentication(payload: &str) -> String {
    blake3::keyed_hash(
        &derive_key("com.walk.tcc-opet/storage-metadata/v1"),
        payload.as_bytes(),
    )
    .to_hex()
    .to_string()
}
