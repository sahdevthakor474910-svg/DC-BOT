use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use rand::seq::SliceRandom;

// ─────────────────────────────────────────────────────────────────────────────
// Cooldown Tracker to Prevent Spam
// ─────────────────────────────────────────────────────────────────────────────

static USER_COOLDOWNS: Mutex<Option<HashMap<u64, Instant>>> = Mutex::new(None);
static CHANNEL_COOLDOWNS: Mutex<Option<HashMap<u64, Instant>>> = Mutex::new(None);

/// Returns `true` if cooldown check passes and updates timestamps.
/// Returns `false` if user or channel was triggered recently (throttled).
pub fn check_and_update_cooldown(user_id: u64, channel_id: u64) -> bool {
    let now = Instant::now();

    // Check user cooldown (4 seconds per user)
    if let Ok(mut lock) = USER_COOLDOWNS.lock() {
        let map = lock.get_or_insert_with(HashMap::new);
        if let Some(&last) = map.get(&user_id) {
            if now.duration_since(last) < Duration::from_secs(4) {
                return false;
            }
        }
        map.insert(user_id, now);
    }

    // Check channel cooldown (2.5 seconds per channel)
    if let Ok(mut lock) = CHANNEL_COOLDOWNS.lock() {
        let map = lock.get_or_insert_with(HashMap::new);
        if let Some(&last) = map.get(&channel_id) {
            if now.duration_since(last) < Duration::from_millis(2500) {
                return false;
            }
        }
        map.insert(channel_id, now);
    }

    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Slang / Bad Words Detection
// ─────────────────────────────────────────────────────────────────────────────

const WHOLE_WORD_SLANGS: &[&str] = &[
    // Short abbreviations
    "bc", "b.c", "b.c.", "mc", "m.c", "m.c.", "bkl", "b.k.l", "bsdk", "b.s.d.k",
    "mkc", "m.k.c", "tmkc", "t.m.k.c", "tmkb", "t.m.k.b", "stfu", "gtfo", "wtf", "cunt",
    // Hindi single-word profanities & slangs
    "gand", "gaand", "gandu", "gaandu", "lodu", "lauda", "lavde", "lawde", "lund", "loda", "laude",
    "randi", "rndi", "rndy", "randy", "randwa", "rndwa", "chut", "choot", "chootiya",
    "saale", "sale", "kamina", "kamine", "harami", "haraami", "kutta", "kutte",
    "suar", "chirkut", "tattu", "jhatu", "jhaatu", "jhant", "bhadwe", "bhadwa",
    // English slangs
    "fuck", "fucker", "fucking", "fucks", "bitch", "bitches", "bastard", "bastards",
    "asshole", "assholes", "dickhead", "dumbass", "pussy", "shithead",
];

const SUBSTRING_SLANGS: &[&str] = &[
    // Short lethal abbreviations
    "tmkc", "bkl", "rndy", "rndi", "randi", "bsdk", "mkc", "tmkb",
    // Desi / Hindi compound swear words
    "bhenchod", "behenchod", "bhen chud", "behen chud", "bhen ke lode", "bhenkelode",
    "behenkelode", "behen ke takke", "madarchod", "madarchot", "maderchod", "madrchod",
    "madarjaat", "maa chuda", "maa chud", "teri maa ki", "maa ki chut", "teri maki chut",
    "maki chut", "teri bhen ki", "teri behen ki", "chutiya", "chutiye", "chutya", "chutiyapa",
    "chutiyapanti", "bhosdike", "bhosadike", "bhosdika", "bhosada", "bhosdiwale", "bhosdiwali",
    "bhosdi", "gandfat", "gandfaad", "gandmasti", "gand mara", "gaand mara", "gand marwa",
    "randibaaz", "randirona",
    // English compound phrases
    "motherfucker", "mother fucker", "bullshit",
];

/// Collapse consecutive duplicate characters: e.g. "fuuuuck" -> "fuck", "bheeeenchoood" -> "bhenchod"
fn collapse_repeats(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_char: Option<char> = None;
    let mut repeat_count = 0;

    for c in s.chars() {
        if Some(c) == prev_char {
            repeat_count += 1;
            // Allow up to 2 repeats (e.g. "ass", "too")
            if repeat_count < 2 {
                result.push(c);
            }
        } else {
            prev_char = Some(c);
            repeat_count = 0;
            result.push(c);
        }
    }

    result
}

/// Collapse all consecutive duplicate characters to a single character: e.g. "fuuuuck" -> "fuck"
fn collapse_to_single(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_char: Option<char> = None;
    for c in s.chars() {
        if Some(c) != prev_char {
            result.push(c);
            prev_char = Some(c);
        }
    }
    result
}

/// Normalize leetspeak and special characters commonly used to mask profanity.
fn normalize_text(text: &str) -> String {
    let lower = text.to_lowercase();
    let replaced: String = lower
        .chars()
        .map(|c| match c {
            '@' => 'a',
            '$' => 's',
            '0' => 'o',
            '1' | '!' => 'i',
            '3' => 'e',
            '*' | '_' | '-' | '~' | '`' => ' ',
            other => other,
        })
        .collect();

    collapse_repeats(&replaced)
}

fn check_words_for_slang(text: &str) -> bool {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    for word in words {
        // Protect "gandhi" from matching "gand"
        if word == "gandhi" || word.starts_with("gandhi") {
            continue;
        }
        // Protect "classic" or "class" from matching "ass"
        if word == "classic" || word == "class" || word == "pass" || word == "grass" || word == "assistant" {
            continue;
        }
        // Protect "because" from matching "bc"
        if word == "because" || word == "bacon" {
            continue;
        }

        for &slang in WHOLE_WORD_SLANGS {
            if word == slang {
                return true;
            }
        }
    }
    false
}

/// Check if a message contains offensive slangs, bad words, or Hindi gali.
pub fn contains_slang(raw_text: &str) -> bool {
    let normalized = normalize_text(raw_text);
    let single_collapsed = collapse_to_single(&normalized);

    // 1. Check substring swear words against normalized and single_collapsed
    for &sub in SUBSTRING_SLANGS {
        if normalized.contains(sub) || single_collapsed.contains(sub) {
            return true;
        }
    }

    // Also check with all spaces removed for evasion like "b h e n c h o d" or "c h u t i y a"
    let no_spaces: String = normalized.chars().filter(|c| !c.is_whitespace()).collect();
    let no_spaces_single = collapse_to_single(&no_spaces);
    for &sub in SUBSTRING_SLANGS {
        if no_spaces.contains(sub) || no_spaces_single.contains(sub) {
            return true;
        }
    }

    // Also check with all non-alphanumeric punctuation removed for evasion like "t.m.k.c", "b.k.l", "b-k-l", "r.n.d.y"
    let no_punct: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
    let no_punct_single = collapse_to_single(&no_punct);
    for &sub in SUBSTRING_SLANGS {
        if no_punct.contains(sub) || no_punct_single.contains(sub) {
            return true;
        }
    }

    // 2. Check token / whole-word matches against both forms
    if check_words_for_slang(&normalized) || check_words_for_slang(&single_collapsed) {
        return true;
    }

    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure Hindi Street Gali & Roasts (For Hindi / Hinglish Messages)
// ─────────────────────────────────────────────────────────────────────────────

const HINDI_GALI_COMEBACKS: &[&str] = &[
    "Ja na lodu, yahan kyu apni marwa raha hai! 🤡",
    "Chal nikal chutiye, pehli fursat mein kat le yahan se! 🚪",
    "Abey bsdk, aukaat mein reh ke baat kar varna yahi pel dunga! 👊",
    "Chup baith bkl, tere jaise lodu ko main roz joota maarta hoon! 👞",
    "Nikal laude, yahan baap ke aage bakchodi mat pel! 🗿",
    "Abey gandu, dimaag bech ke internet pack karwaya hai kya lodu? 🧠",
    "Chal hatt lodu, tere bolne se kutta bhi na dare! 🐶",
    "Chup kar chutiye, muh se hagte ho ya dimaag se? 💀",
    "Abey madarchod, limit mein reh varna yahi patak ke pelunga! ⚡",
    "Bhenchod dimaag mat kharab kar, nikal yahan se! 🔥",
    "Jaake apni gaand mara lodu, yahan gyan mat pel! 🎪",
    "Abey jhatu chutiye, aukaat dekh ke gali de, yahan sab tere baap baithe hain! 👑",
    "Chal be lodu, aage badh, faltu ka bheed mat kar! 🛵",
    "Chup be laude, tere jaise 36 aate hain daily rona dhona karne! 🥱",
    "Abey bhadwe, teri do rupaye ki aukaat nahi aur gali de raha hai chutiye! 💸",
    "Nikal bkl, yahan beizzati karwane ka itna shauk hai kya? 😂",
    "Abey saale harami chutiye, ek thappad mein saara chutiyapa nikal dunga! 💥",
    "Oye bsdk lodu, mute kar du kya ya khud apni aukaat mein aayega? 🔇",
    "Ja na lodu, pehle homework kar le phir aana chaudha hone! 🍼",
    "Bhenchod chup chap baith ja, faltu mein kutte ki maut marega chutiye! ☠️",
    "Chal nikal gandu, tere jaise nalle ko reply dena bhi beizzati hai! 🗑️",
    "Muh band rakh laude, varna aisi beizzati karunga ki Discord delete kar dega! 📱",
    "Abey chutiye lodu, internet mila toh khud ko don samajh raha hai kya? 🤡",
    "Ja na bkl, jahan se nikla hai wahi wapas ghus ja! 🕳️",
    "Teri aisi taisi lodu, chal pehli fursat mein nikal yahan se! 🚶",
    "Chup kar madarchod, baap ke aage aisi baat karega toh laat padegi! 🩴",
    "Abey gandu, itni kyu jal rahi hai teri? Gaand pe baraf laga le jaake! 🧊",
    "Chal chutiye, tujhe toh gali dena bhi gali ki beizzati hai lodu! 🤮",
    "Laude chup baith, dimaag ka bhosda mat kar yahan! 🤯",
    "Chup bsdk, tere jaiso ko main nashte mein khaa jata hoon! 🍳",
    "Abey lallu chutiye, yahan apna randirona band kar! 😭",
    "Ja na laude, aukaat nahi hai saamne khade hone ki aur internet pe sher ban raha hai! 🦁",
    "Bhenchod itna uchhal mat, sadak pe ghis dunga! 🏍️",
    "Abey lodu, tere khandaan mein kisi ne tameez sikhaayi hai ya sab tere jaise chutiye the? 🤡",
    "Oye madarchod, dhang se bol varna aisi gaali dunga ki 7 pushte yaad rakhengi! 🧨",
    "Chal nikal lodu, yahan apna chutiyapa failana band kar! 🧹",
    "Abey bkl gandu, teri aukaat utni hi hai jitni chips ke packet mein hawa hoti hai! 🎈",
    "Muh sambhal ke bol chutiye, varna yahi tera dhuan nikal dunga! 💨",
    "Bhenchod chup! Ek shabd aur bola toh yahi pe gaand laal kar dunga! 🔴",
    "Ja na lodu, jaake pogo dekh, yahan baap ke saamne bakwaas mat kar! 📺",
    "Abey chutiye, zabaan pe control rakh warna kheench ke baahar nikal dunga! 👅",
    "Chal be gandu, do kaudi ka insaan aur baatein aasmaan ki! ⚡",
    "Tere jaise lodu ko toh mirror dekh ke bhi ulti aati hogi, chup kar ab! 🪞",
    "Bhenchod khud ki shakal dekhi hai kabhi? Pehle dhang se type karna seekh le gawaar! ⌨️",
    "Ja na laude, kisi aur ke samne apni hekdhi dikhana, yahan nahi chalegi! 🚫",
    "Abey bkl, dimaag ghutne mein hai kya jo har jagah apni beizzati karwata hai? 🦵",
    "Aukaat mein reh lodu, samjha na? Agli baar bola toh nanga karke pelunga! ⚠️",
    "Chup baith madarchod, teri baaton se sabka sar dard ho raha hai! 🤕",
    "Abey saale chutiye, tere aane se pehle shanti thi, pura kachra kar diya aake! 🗑️",
    "Chal kat le lodu, tere se baat karna matlab kachre ke dabbe mein patthar maarna! 🪨",
    // Comebacks with user-requested slangs (tmkc, bkl, rndy)
    "Chal nikal tmkc, yahan apna randirona band kar! 🚪",
    "Abey bkl, aukaat dekh ke baat kar varna yahi pel dunga! 👊",
    "Chup baith bkl rndy, tere jaise nalle ko roz joota maarta hoon! 👞",
    "Chal be tmkc, do rupaye ki aukaat nahi aur yahan gali pel raha hai! 💸",
    "Abey bkl, muh band rakh varna aisi beizzati karunga ki Discord delete kar dega! 📱",
    "Ja na rndy, kisi aur ke aage rona, yahan sab tere baap baithe hain! 👑",
    "Chal nikal tmkc, tere bolne se kutta bhi na dare lodu! 🐶",
    "Abey bkl chutiye, dimaag bech ke internet pack karwaya hai kya? 🧠",
    "Oye rndy ke pille, limit mein reh varna yahi patak ke dho dunga! ⚡",
    "Chal hatt bkl, baap ke aage bakchodi karega toh gaand laal kar dunga! 🔴",
    "Tmkc nikal yahan se, faltu mein kutte ki maut marega! ☠️",
    "Abey bkl rndy, shakal dekhi hai aaine mein? Jaake pogo dekh lodu! 📺",
    "Chal nikal bkl, tere jaise 36 aate hain daily yahan rona dhona karne! 🥱",
    "Tmkc chup baith, dimaag ka bhosda mat kar yahan! 🤯",
    "Abey bkl, itni kyu jal rahi hai teri? Baraf laga le jaake! 🧊",
    "Nikal tmkc, yahan beizzati karwane ka subscription le rakha hai kya bkl? 😂",
    "Chup kar rndy, baap se bakwaas karega toh yahi zameen mein gaad dunga! 🕳️",
    "Tmkc aukaat mein reh, ek chamaat mein saara chutiyapa nikal dunga! 💥",
    "Abey bkl, dimaag ghutne mein hai kya jo har jagah apni gaand marwata hai? 🦵",
    "Chal kat le rndy, tere jaise chutiye ko reply dena bhi time waste hai! ⏱️",
];

// ─────────────────────────────────────────────────────────────────────────────
// Pure 2026 Gen Z Discord Roasts (For English Messages)
// ─────────────────────────────────────────────────────────────────────────────

const GENZ_ENGLISH_COMEBACKS: &[&str] = &[
    "Lil bro really thought he did something 💀 sit down.",
    "Bro is yapping with negative aura 😭",
    "Blud thinks he's the main character 💀 who invited you?",
    "Bro cooked absolutely nothing, pure clown behavior 🤡",
    "Major L + ratio + touch grass 💀",
    "Bro is tweaking over Discord pixels 😭 go outside.",
    "0 rizz, -1000000 aura, 100% cringe 💀",
    "Bro really typed that and felt like a sigma 😭 embarrassing.",
    "Lil bro's whole personality is crying in chat 💀 pipe down.",
    "Chat is this real? Bro really thought he had motion 🤡",
    "Blud is cooked beyond recognition 💀 log off.",
    "Bro is yapping a whole essay 😭 yap session is over.",
    "Ain't no way bro is flexing with zero motion 💀",
    "NPC dialogue detected 🤖 keep scrolling.",
    "Bro really fumbled the comeback 😭 delete the message.",
    "Bro has negative IQ and zero rizz 💀 do your homework first.",
    "Blud's downfall needs to be studied 📉",
    "Bro really thought he ate that 💀 hungry for attention.",
    "Certified yapper of the year award goes to you 🏆",
    "Bro got pressed by text on a screen 😭 touch grass.",
    "Ain't reading all that yap lil bro 💀 bye.",
    "Bro's aura evaporated in 2 seconds 💀",
    "Blud is fighting for his life in chat 😭 embarrassing.",
    "Bro is down bad and tweaking out 😭 take a breath.",
    "Bro thinks he's the final boss 💀 one tap and you're done.",
    "Bro's confidence is way higher than his IQ 💀",
    "Delulu is the only solulu for bro 💀 get real.",
    "Blud really dropped the most NPC line ever 😭",
    "Bro is crying in 4K 120FPS 😭 go complain to your mom.",
    "Ain't no way lil bro is barking on Discord 💀 mute yourself.",
];

const HINDI_DETECT_WORDS: &[&str] = &[
    "bc", "mc", "bkl", "bsdk", "mkc", "tmkc", "tmkb",
    "bhenchod", "behenchod", "madarchod", "madarchot", "maderchod",
    "chutiya", "chutiye", "chutya", "chootiya", "chutiyapa",
    "bhosdike", "bhosadike", "bhosdika", "bhosada", "bhosdi",
    "gandu", "gaandu", "lodu", "lauda", "lavde", "lawde", "lund", "loda", "laude",
    "randi", "rndi", "rndy", "randy", "randwa", "rndwa", "harami", "kamine", "kamina",
    "chirkut", "tattu", "jhatu", "jhaatu", "jhant", "suar", "bhadwe", "bhadwa",
    "maa chuda", "teri maa", "maa ki", "teri bhen", "teri behen",
    "kya", "hai", "hain", "tera", "teri", "tere", "mera", "meri", "mere",
    "tu", "tum", "aap", "kar", "karo", "raha", "rahe", "rahi", "chal",
    "nikal", "ja", "jana", "abey", "oye", "bhai", "yaar", "saale", "sale",
    "aukaat", "baap", "dimaag", "chup", "pel", "pelunga", "baraf", "kutta",
];

/// Determine if a message is in Hindi / Hinglish.
pub fn is_hindi_message(text: &str) -> bool {
    let lower = text.to_lowercase();

    // Check substring indicators
    for &sub in &[
        "bhenchod", "behenchod", "madarchod", "bhosdike", "chutiya",
        "teri maa", "maa ki", "bhen ke", "gand mara", "chutiyapa", "chup kar",
        "tmkc", "bkl", "rndy", "rndi", "randi", "bsdk", "mkc", "tmkb",
    ] {
        if lower.contains(sub) {
            return true;
        }
    }

    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    for w in words {
        if w == "because" || w == "bacon" || w == "classic" {
            continue;
        }
        for &hw in HINDI_DETECT_WORDS {
            if w == hw {
                return true;
            }
        }
    }

    false
}

/// Pick an appropriate comeback based on message language:
/// - Hindi slang/message -> Pure Hindi street comeback
/// - English message -> Pure 2026 Gen Z English roast
pub fn get_comeback_for_message(message: &str) -> &'static str {
    let mut rng = rand::thread_rng();
    if is_hindi_message(message) {
        HINDI_GALI_COMEBACKS.choose(&mut rng).copied().unwrap_or(
            "Ja na lodu, aukaat dekh ke baat kar! 🤡",
        )
    } else {
        GENZ_ENGLISH_COMEBACKS.choose(&mut rng).copied().unwrap_or(
            "Lil bro really thought he did something 💀 sit down.",
        )
    }
}

/// Pick a random comeback (50/50 Hindi or Gen Z English)
pub fn get_random_gali_comeback() -> &'static str {
    let mut rng = rand::thread_rng();
    let is_hindi: bool = rand::random();
    if is_hindi {
        HINDI_GALI_COMEBACKS.choose(&mut rng).copied().unwrap_or(
            "Ja na lodu, aukaat dekh ke baat kar! 🤡",
        )
    } else {
        GENZ_ENGLISH_COMEBACKS.choose(&mut rng).copied().unwrap_or(
            "Lil bro really thought he did something 💀 sit down.",
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slang_detection_hindi() {
        assert!(contains_slang("abey bhenchod chup kar"));
        assert!(contains_slang("kya be madarchod"));
        assert!(contains_slang("chutiya ho kya"));
        assert!(contains_slang("chutiye"));
        assert!(contains_slang("bsdk"));
        assert!(contains_slang("nikal bkl"));
        assert!(contains_slang("teri maa ki choot"));
        assert!(contains_slang("bhosdike"));
        assert!(contains_slang("tu gandu hai"));
        assert!(contains_slang("lavde nikal"));
        assert!(contains_slang("lodu"));
        // User requested: tmkc, bkl, rndy, etc.
        assert!(contains_slang("tmkc"));
        assert!(contains_slang("bkl"));
        assert!(contains_slang("rndy"));
        assert!(contains_slang("chal nikal tmkc"));
        assert!(contains_slang("abey bkl chup baith"));
        assert!(contains_slang("ja na rndy"));
    }

    #[test]
    fn test_slang_detection_english() {
        assert!(contains_slang("what the fuck is this"));
        assert!(contains_slang("shut up bitch"));
        assert!(contains_slang("you motherfucker"));
        assert!(contains_slang("dumbass"));
        assert!(contains_slang("stfu"));
    }

    #[test]
    fn test_slang_detection_leetspeak() {
        assert!(contains_slang("fuuuck"));
        assert!(contains_slang("chuuuutiyapa"));
        assert!(contains_slang("b h e n c h o d"));
        // Punctuation and spacing evasion for abbreviations
        assert!(contains_slang("t.m.k.c"));
        assert!(contains_slang("b.k.l"));
        assert!(contains_slang("r.n.d.y"));
        assert!(contains_slang("b.s.d.k"));
        assert!(contains_slang("t-m-k-c"));
        assert!(contains_slang("b_k_l"));
        assert!(contains_slang("t m k c"));
        assert!(contains_slang("b k l"));
        assert!(contains_slang("r n d y"));
    }

    #[test]
    fn test_false_positives() {
        assert!(!contains_slang("Mahatma Gandhi is a historical figure"));
        assert!(!contains_slang("I like classic cars"));
        assert!(!contains_slang("Please pass the salt"));
        assert!(!contains_slang("I am doing this because I want to"));
        assert!(!contains_slang("Hello bot how are you today"));
    }

    #[test]
    fn test_random_comeback() {
        let comeback = get_random_gali_comeback();
        assert!(!comeback.is_empty());
    }

    #[test]
    fn test_separated_comebacks() {
        // Hindi messages must get pure Hindi comebacks
        assert!(is_hindi_message("abey bhenchod"));
        assert!(is_hindi_message("chal nikal lodu"));
        assert!(is_hindi_message("tmkc"));
        assert!(is_hindi_message("bkl"));
        assert!(is_hindi_message("rndy"));
        assert!(is_hindi_message("chal nikal tmkc"));
        let hindi_reply = get_comeback_for_message("abey chutiye chup kar");
        assert!(HINDI_GALI_COMEBACKS.contains(&hindi_reply));
        let tmkc_reply = get_comeback_for_message("tmkc");
        assert!(HINDI_GALI_COMEBACKS.contains(&tmkc_reply));

        // English messages must get pure Gen Z English comebacks
        assert!(!is_hindi_message("what the fuck"));
        assert!(!is_hindi_message("shut up you bitch"));
        let english_reply = get_comeback_for_message("shut up bitch");
        assert!(GENZ_ENGLISH_COMEBACKS.contains(&english_reply));
    }
}
