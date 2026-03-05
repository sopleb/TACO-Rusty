// Data
pub const SYSTEMDATA_JSON: &str = include_str!("../resources/data/systemdata.json");
pub const REGIONS_JSON: &str = include_str!("../resources/data/regions.json");

// Shaders
pub const SHADER_VERT: &str = include_str!("../resources/shaders/shader.vert");
pub const SHADER_FRAG: &str = include_str!("../resources/shaders/shader.frag");
pub const CONNECTION_VERT: &str = include_str!("../resources/shaders/connection.vert");
pub const CONNECTION_FRAG: &str = include_str!("../resources/shaders/connection.frag");
pub const CROSSHAIR_VERT: &str = include_str!("../resources/shaders/crosshair.vert");
pub const CROSSHAIR_FRAG: &str = include_str!("../resources/shaders/crosshair.frag");

// Textures
pub const TEX_SYSTEM: &[u8] = include_bytes!("../resources/icons/system.png");
pub const TEX_GREEN_CH: &[u8] = include_bytes!("../resources/icons/green-crosshair.png");
pub const TEX_RED_CH: &[u8] = include_bytes!("../resources/icons/red-crosshair.png");
pub const TEX_YELLOW_CH: &[u8] = include_bytes!("../resources/icons/yellow-crosshair.png");
pub const TEX_RED_GREEN_CH: &[u8] = include_bytes!("../resources/icons/redgreen-crosshair.png");
pub const TEX_RED_YELLOW_CH: &[u8] = include_bytes!("../resources/icons/redyellow-crosshair.png");
pub const TEX_YELLOW_GREEN_CH: &[u8] = include_bytes!("../resources/icons/yellowgreen-crosshair.png");
pub const TEX_ICON: &[u8] = include_bytes!("../resources/icons/icon-512-maskable.png");

// Sounds
pub const SOUND_1UP1: &[u8] = include_bytes!("../resources/sounds/1up1.wav");
pub const SOUND_BOO2: &[u8] = include_bytes!("../resources/sounds/Boo2.wav");
pub const SOUND_COIN: &[u8] = include_bytes!("../resources/sounds/Coin.wav");
pub const SOUND_KAMEK_LAUGH: &[u8] = include_bytes!("../resources/sounds/KamekLaugh.wav");
pub const SOUND_POWERUP: &[u8] = include_bytes!("../resources/sounds/Powerup.wav");
pub const SOUND_RED_COIN2: &[u8] = include_bytes!("../resources/sounds/RedCoin2.wav");
pub const SOUND_RED_COIN3: &[u8] = include_bytes!("../resources/sounds/RedCoin3.wav");
pub const SOUND_STAR_COIN: &[u8] = include_bytes!("../resources/sounds/StarCoin.wav");
pub const SOUND_SUIT_FLY: &[u8] = include_bytes!("../resources/sounds/SuitFly.wav");
pub const SOUND_SUIT_SPIN: &[u8] = include_bytes!("../resources/sounds/SuitSpin.wav");
pub const SOUND_WHISTLE: &[u8] = include_bytes!("../resources/sounds/Whistle.wav");
pub const SOUND_CALL_INSIDE_HOUSE: &[u8] = include_bytes!("../resources/sounds/CallInsideHouse.wav");
pub const SOUND_HOSTILES_1JUMP: &[u8] = include_bytes!("../resources/sounds/Hostiles1jump.wav");
pub const SOUND_HOSTILES_2JUMP: &[u8] = include_bytes!("../resources/sounds/hostiles2jump.wav");
pub const SOUND_HOSTILES_3JUMP: &[u8] = include_bytes!("../resources/sounds/hostiles3jump.wav");
pub const SOUND_HOSTILES_4JUMP: &[u8] = include_bytes!("../resources/sounds/hostiles4jump.wav");
pub const SOUND_HOSTILES_HERE: &[u8] = include_bytes!("../resources/sounds/HostilesHere.wav");

pub fn get_sound_data(name: &str) -> Option<&'static [u8]> {
    match name {
        "1up1" => Some(SOUND_1UP1),
        "Boo2" => Some(SOUND_BOO2),
        "Coin" => Some(SOUND_COIN),
        "KamekLaugh" => Some(SOUND_KAMEK_LAUGH),
        "Powerup" => Some(SOUND_POWERUP),
        "RedCoin2" => Some(SOUND_RED_COIN2),
        "RedCoin3" => Some(SOUND_RED_COIN3),
        "StarCoin" => Some(SOUND_STAR_COIN),
        "SuitFly" => Some(SOUND_SUIT_FLY),
        "SuitSpin" => Some(SOUND_SUIT_SPIN),
        "Whistle" => Some(SOUND_WHISTLE),
        "CallInsideHouse" => Some(SOUND_CALL_INSIDE_HOUSE),
        "Hostiles1jump" => Some(SOUND_HOSTILES_1JUMP),
        "Hostiles2jump" => Some(SOUND_HOSTILES_2JUMP),
        "Hostiles3jump" => Some(SOUND_HOSTILES_3JUMP),
        "Hostiles4jump" => Some(SOUND_HOSTILES_4JUMP),
        "HostilesHere" => Some(SOUND_HOSTILES_HERE),
        _ => None,
    }
}
