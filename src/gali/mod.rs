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
    "Tujhse behtar comeback toh auto-correct de deta hai bkl.",
    "Pehli baar internet mila hai kya jo yahan itna uchhal raha hai chutiye?",
    "Itna vella baitha hai ki chat mein aake beizzati karwa raha hai lodu.",
    "Aukaat do rupaye ki nahi hai aur baatein aasmaan ki pel raha hai bsdk.",
    "Tera dimaag offline hai ya bachpan se hi software crash hai bkl?",
    "Shakal aisi hai ki aaina bhi dekh ke error dikha de, aur akad dekho tmkc.",
    "Do line bolne ki aukaat nahi aur yahan gyan baantne chala hai chutiya.",
    "Tujhe lagta hai tu cool lag raha hai? Joker lag raha hai lawde.",
    "Aisa joota padega na ki 3 din tak date aur time bhool jayega bkl.",
    "Chup baith bsdk, tere jaise sadak chhaap roz gali sun ke jaate hain.",
    "Bina dimaag ke bolne ki aadat daal li hai kya tune gandu?",
    "Dhang se type karna sikh pehle, phir aana baap se behas karne laude.",
    "Pura khandaan milke bhi tere liye ek dhang ka dimaag nahi la sakta chutiye.",
    "Yahan aake apna randirona mat pel, chup chap nikal le tmkc.",
    "Teri bakwaas sun ke lag raha hai school ka fees barbaad kiya tere baap ne.",
    "Itna confidence late kahan se ho be? Do kaudi ki akal nahi hai tere paas.",
    "Zabaan sambhal ke baat kar varna yahi aukaat dikha dunga bkl.",
    "Khud ka career bana pehle, yahan chat mein sher mat ban lodu.",
    "Tujhe reply dena matlab kachre ke dibbe mein patthar phekna hai.",
    "Aaina dekh le ek baar, saari akad 2 second mein hawa ho jayegi bsdk.",
    "Aankhein khol ke dekh, koi tujhe poochh bhi nahi raha yahan pe rndy.",
    "Ghar walo ko pata hai tu internet pe aisi beizzati karwata hai chutiye?",
    "Ek thappad mein saari hawa nikal jayegi, zyada udd mat lawde.",
    "Jitna dimaag tere paas hai, utne mein toh calculator bhi nahi chalta bkl.",
    "Chup chap kone mein baith ja, faltu ka scene mat create kar tmkc.",
    "Tujhe dekh ke lagta hai bhagwan akal dete time tujhe bhool gaya tha.",
    "Baat karne ki tameez nahi hai aur aa gaya yahan chaude mein bsdk.",
    "Jo bolna hai dhang se bol, varna pehli fursat mein patak ke pelunga lodu.",
    "Tujhe laga tu bohot bada teer maar liya? Khota sikka hai tu chutiye.",
    "Aisi gaaliyan sunega na ki dictionary choti pad jayegi, isliye chup reh bkl.",
    "Tera ye roz ka rona dekh ke taras aata hai teri zindagi pe rndy.",
    "Aage badh lawde, yahan faltu bheed mat badha.",
    "Tere jaise nallo ki wajah se hi internet sasta hona band hona chahiye tmkc.",
    "Muh se kachra nikalna band kar aur dhang ki baat seekh bsdk.",
    "Lagta hai subah se kisi ne zillat nahi di tujhe, isliye yahan aa gaya chutiye.",
    "Limit cross mat kar, varna aisi beizzati hogi ki account delete karega bkl.",
    "Tere bolne ka koi matlab nahi hai, bilkul mute ho ja lodu.",
    "Akal ghaas charne gayi hai ya paidaishi aise hi namoone ho?",
    "Aukaat mein reh ke bol varna chat se aisi laat padegi ki yaad rakhega tmkc.",
    "Chal kat le, tere se muh lagna bhi time kharaab karna hai.",
    "Har jagah aake apni beizzati karwana compulsory hai kya tera bsdk?",
    "Aadha kilo dimaag khareed le jaake, yahan be-matlab ki ulti mat kar laude.",
    "Tere logic dekh ke lagta hai dimaag ke naam pe ghutna fit hai chutiye.",
    "Bina kisi aukaat ke chat mein bakwaas karna band kar bkl rndy.",
    "Jitna chilla raha hai na, utne mein tera hi blood pressure badhega lodu.",
    "Tu rehne de bhai, tere bas ki baat hi nahi hai dhang se baat karna.",
    "Bhenchod chup chap nikal, yahan sab tera tamasha dekh rahe hain.",
    "Aise baatein kar raha hai jaise bohot bada don ho, asal mein do rupaye ka chirkut hai.",
    "Teri aukat sirf screen ke peeche se bhaukne ki hai bsdk.",
    "Chal nikal, tere jaiso ko bhaav dena hi sabse badi galti hai.",
    "Agle janam mein thoda dimaag leke paida hona lodu.",
    "Bhenchod shakal se gawaar aur baaton se nalla lagta hai tu.",
    "Chat mein sher aur real life mein kutte ki tarah darta hai chutiye.",
    "Tmkc itna shauk hai beizzati ka toh mirror ke samne khade ho ja.",
    "Aukaat dekh ke pange liya kar varna sadak pe ghis ke pel dunga bkl.",
    "Tera pura wajood hi ek galat decision lagta hai rndy.",
    "Apna ye ghatiya attitude leke kisi aur ke aage ja, yahan tera baap khada hai.",
    "Do shabd dhang se bol nahi paata aur gali sikh ke hero ban raha hai bsdk.",
    "Bina sir pair ki baatein karna band kar aur chup chap dafa ho ja lodu.",
    "Tujhse baat karke mere hi neurons mar rahe hain chutiye.",
];

// ─────────────────────────────────────────────────────────────────────────────
// Creative, Diverse Modern Internet Roasts (For English)
// ─────────────────────────────────────────────────────────────────────────────

const GENZ_ENGLISH_COMEBACKS: &[&str] = &[
    "You really spent precious minutes of your life typing that out 😭🙏",
    "Not a single thought went into that sentence and it genuinely shows 💀",
    "Delete this before anyone else sees how embarrassing you are 🥀",
    "Talking with this much confidence with zero facts to back it up 🫵😂",
    "Your screen time needs to be revoked immediately 📉",
    "Imagine getting cooked this bad in public chat 😭🙏",
    "You're fighting an uphill battle with a severe IQ disadvantage 💀",
    "Even the bots in this server are feeling second-hand embarrassment for you 🥀",
    "Log out, reconsider your life choices, and try again tomorrow 🪫",
    "Confidence at 100%, competence sitting at absolute zero 📉",
    "Whoever told you that was a good comeback lied directly to your face 💀",
    "You typed all that just to deliver absolutely zero impact 🫵😂",
    "Every time you speak the collective IQ of this channel takes a nosedive 📉",
    "You're loud and completely wrong, pick a struggle 😭🙏",
    "Please don't hurt yourself trying to formulate another sentence 💀",
    "That insult had the devastating impact of a wet paper towel 🥀",
    "You really thought you did something special there 🫵😂",
    "There's still time to pretend your cat walked across your keyboard 😭🙏",
    "Standing on business with absolutely no business to stand on 📉",
    "You're one bad argument away from completely deactivating your account 💀",
    "Taking an L of this magnitude in public takes genuine talent 🥀",
    "The silence from everyone else was your cue to pack it up 🫵😂",
    "You're arguing for third place in a two-man race 😭🙏",
    "Zero substance, zero accuracy, 100% emotional damage 📉",
    "Wrap it up, the audience has completely checked out 🚪💀",
    "Your Wi-Fi is doing all that heavy lifting just for you to post garbage 🥀",
    "Self-awareness has completely abandoned you today 😭🙏",
    "I've seen smarter arguments come out of an auto-correct glitch 💀",
    "You really felt proud pressing send on that mess 🫵😂",
    "You're swinging at shadows in the dark and still missing 📉",
    "Don't flatter yourself, nobody was intimidated by that nonsense 💀",
    "You're embarrassing yourself for free when you could just stay quiet 😭🙏",
    "That comeback expired back in 2016, update your material 🥀",
    "Speechless from how tragic that attempt at an insult was 🫵😂",
    "You woke up today and chose pure unprovoked public embarrassment 💀",
    "Go touch some real grass, the digital world is chewing you up 📉",
    "Your arguments are held together by Scotch tape and wishful thinking 😭🙏",
    "You brought butter knives to a verbal gunfight 💀",
    "Mute yourself before you dig an even deeper hole 🪫",
    "A masterclass in saying a lot of words while making zero points 🫵😂",
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

static RECENT_COMEBACKS: Mutex<Option<std::collections::VecDeque<&'static str>>> = Mutex::new(None);

/// Pick a comeback avoiding recently chosen ones to eliminate repetitive lines
fn pick_non_recent(pool: &[&'static str]) -> &'static str {
    let mut rng = rand::thread_rng();
    if let Ok(mut lock) = RECENT_COMEBACKS.lock() {
        let queue = lock.get_or_insert_with(std::collections::VecDeque::new);

        let candidates: Vec<&'static str> = pool
            .iter()
            .copied()
            .filter(|item| !queue.contains(item))
            .collect();

        let chosen = if !candidates.is_empty() {
            *candidates.choose(&mut rng).unwrap_or(&pool[0])
        } else {
            queue.clear();
            *pool.choose(&mut rng).unwrap_or(&pool[0])
        };

        queue.push_back(chosen);
        if queue.len() > 15 {
            queue.pop_front();
        }

        chosen
    } else {
        *pool.choose(&mut rng).unwrap_or(&pool[0])
    }
}

/// Pick an appropriate comeback based on message language:
/// - Hindi slang/message -> Pure Hindi street comeback
/// - English message -> Creative, sharp English roast
pub fn get_comeback_for_message(message: &str) -> &'static str {
    if is_hindi_message(message) {
        pick_non_recent(HINDI_GALI_COMEBACKS)
    } else {
        pick_non_recent(GENZ_ENGLISH_COMEBACKS)
    }
}

/// Pick a random comeback (50/50 Hindi or English)
pub fn get_random_gali_comeback() -> &'static str {
    let is_hindi: bool = rand::random();
    if is_hindi {
        pick_non_recent(HINDI_GALI_COMEBACKS)
    } else {
        pick_non_recent(GENZ_ENGLISH_COMEBACKS)
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
