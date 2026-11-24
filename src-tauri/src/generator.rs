use crate::{
    error::{AppError, AppResult},
    models::{GeneratedSecret, PassphraseOptions, PasswordOptions},
};
use rand::{seq::IndexedRandom, seq::SliceRandom, Rng};

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?/";
const SIMILAR: &[char] = &['0', 'O', '1', 'l', 'I', '|', 'i'];

pub const WORDS: &[&str] = &[
    "amber", "anchor", "arbor", "atlas", "audio", "azure", "badge", "basil", "bayou", "beacon",
    "birch", "bloom", "bridge", "brook", "cactus", "canyon", "cedar", "comet", "coral", "cosmic",
    "crater", "crystal", "dawn", "delta", "dense", "desert", "dune", "ember", "falcon", "fern",
    "fjord", "flame", "flare", "frost", "galaxy", "garden", "glade", "glow", "granite", "grove",
    "harbor", "hazel", "heron", "hill", "hollow", "horizon", "hut", "indigo", "inlet", "iris",
    "island", "ivory", "jade", "jungle", "juniper", "kestrel", "kiln", "kindle", "lagoon",
    "lantern", "laurel", "ledge", "lemon", "lighthouse", "linen", "lunar", "maple", "meadow",
    "mist", "mosaic", "moss", "mountain", "nebula", "nectar", "nest", "night", "north", "oak",
    "oasis", "ocean", "olive", "orbit", "orchard", "pacific", "palm", "panther", "papyrus",
    "pebble", "pepper", "pine", "plateau", "prism", "quartz", "quest", "quiet", "raven", "reef",
    "ridge", "river", "robin", "root", "sable", "saffron", "sage", "sand", "saturn", "savanna",
    "sequoia", "shadow", "shore", "silver", "slate", "solar", "sonar", "spruce", "storm",
    "summit", "sun", "tidal", "timber", "topaz", "torch", "trail", "tranquil", "tulip", "umber",
    "valley", "velvet", "verdant", "violet", "walnut", "willow", "winter", "wired", "wonder",
    "xenon", "yarrow", "yellow", "zenith", "zephyr", "zircon",
];

fn filtered(pool: &str, exclude_similar: bool) -> Vec<char> {
    pool.chars()
        .filter(|c| !exclude_similar || !SIMILAR.contains(c))
        .collect()
}

pub fn generate_password(options: &PasswordOptions) -> AppResult<GeneratedSecret> {
    if options.length < 4 || options.length > 256 {
        return Err(AppError::new("Длина пароля должна быть от 4 до 256"));
    }

    let mut pools: Vec<Vec<char>> = Vec::new();
    if options.lowercase {
        pools.push(filtered(LOWER, options.exclude_similar));
    }
    if options.uppercase {
        pools.push(filtered(UPPER, options.exclude_similar));
    }
    if options.digits {
        pools.push(filtered(DIGITS, options.exclude_similar));
    }
    if options.symbols {
        pools.push(filtered(SYMBOLS, options.exclude_similar));
    }

    let mut combined = Vec::new();
    for pool in &pools {
        for ch in pool {
            if !combined.contains(ch) {
                combined.push(*ch);
            }
        }
    }
    if combined.is_empty() {
        return Err(AppError::new("Выберите хотя бы один набор символов"));
    }
    if pools.len() > options.length {
        return Err(AppError::new("Длина пароля меньше количества наборов символов"));
    }

    let mut rng = rand::rng();
    let mut chars: Vec<char> = pools
        .iter()
        .filter_map(|pool| pool.choose(&mut rng).copied())
        .collect();

    while chars.len() < options.length {
        if let Some(ch) = combined.choose(&mut rng) {
            chars.push(*ch);
        }
    }
    chars.shuffle(&mut rng);

    let entropy_bits = options.length as f64 * (combined.len() as f64).log2();
    Ok(GeneratedSecret {
        value: chars.into_iter().collect(),
        entropy_bits,
        kind: "password".to_string(),
    })
}

pub fn generate_passphrase(options: &PassphraseOptions) -> AppResult<GeneratedSecret> {
    if options.words < 2 || options.words > 24 {
        return Err(AppError::new("Количество слов должно быть от 2 до 24"));
    }
    let separator = if options.separator.is_empty() {
        "-".to_string()
    } else {
        options.separator.clone()
    };

    let mut rng = rand::rng();
    let mut words: Vec<String> = Vec::with_capacity(options.words);
    for _ in 0..options.words {
        let word = WORDS
            .choose(&mut rng)
            .copied()
            .ok_or_else(|| AppError::new("Словарь пуст"))?;
        if options.capitalize {
            let mut chars = word.chars();
            let capitalized = chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_else(|| word.to_string());
            words.push(capitalized);
        } else {
            words.push(word.to_string());
        }
    }

    let mut value = words.join(&separator);
    let mut entropy_bits = options.words as f64 * (WORDS.len() as f64).log2();
    if options.append_digit {
        value.push_str(&rng.random_range(0..10).to_string());
        entropy_bits += (10f64).log2();
    }

    Ok(GeneratedSecret {
        value,
        entropy_bits,
        kind: "passphrase".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn password_options(length: usize, exclude_similar: bool) -> PasswordOptions {
        PasswordOptions {
            length,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_similar,
        }
    }

    #[test]
    fn password_has_requested_length_and_all_selected_sets() {
        let secret = generate_password(&password_options(32, false)).unwrap();
        assert_eq!(secret.value.chars().count(), 32);
        assert!(secret.value.chars().any(|c| c.is_ascii_lowercase()));
        assert!(secret.value.chars().any(|c| c.is_ascii_uppercase()));
        assert!(secret.value.chars().any(|c| c.is_ascii_digit()));
        assert!(secret.value.chars().any(|c| !c.is_ascii_alphanumeric()));
        assert_eq!(secret.kind, "password");
    }

    #[test]
    fn excluding_similar_characters_removes_ambiguous_symbols() {
        for _ in 0..20 {
            let secret = generate_password(&password_options(48, true)).unwrap();
            assert!(secret.value.chars().all(|c| !SIMILAR.contains(&c)));
        }
    }

    #[test]
    fn entropy_grows_with_length_and_pool_size() {
        let short = generate_password(&password_options(8, false)).unwrap();
        let long = generate_password(&password_options(24, false)).unwrap();
        assert!(long.entropy_bits > short.entropy_bits);

        let mut digits_only = password_options(24, false);
        digits_only.uppercase = false;
        digits_only.lowercase = false;
        digits_only.symbols = false;
        let digits = generate_password(&digits_only).unwrap();
        assert!(long.entropy_bits > digits.entropy_bits);
    }

    #[test]
    fn invalid_password_options_are_rejected() {
        assert!(generate_password(&password_options(2, false)).is_err());
        assert!(generate_password(&password_options(300, false)).is_err());

        let mut none = password_options(16, false);
        none.uppercase = false;
        none.lowercase = false;
        none.digits = false;
        none.symbols = false;
        assert!(generate_password(&none).is_err());

        let mut too_many_pools = password_options(2, false);
        too_many_pools.symbols = true;
        assert!(generate_password(&too_many_pools).is_err());
    }

    #[test]
    fn passphrase_respects_words_separator_and_capitalization() {
        let options = PassphraseOptions {
            words: 5,
            separator: "~".to_string(),
            capitalize: true,
            append_digit: true,
        };
        let secret = generate_passphrase(&options).unwrap();
        let parts: Vec<&str> = secret.value.split('~').collect();
        assert_eq!(parts.len(), 5);
        assert!(parts.iter().all(|part| part.chars().next().unwrap().is_uppercase()));
        assert!(secret.value.chars().last().unwrap().is_ascii_digit());
        assert_eq!(secret.kind, "passphrase");
        assert!(secret.entropy_bits > 5.0 * 6.0);
    }

    #[test]
    fn passphrase_rejects_word_counts_outside_range() {
        for words in [1, 25] {
            let options = PassphraseOptions {
                words,
                separator: "-".to_string(),
                capitalize: false,
                append_digit: false,
            };
            assert!(generate_passphrase(&options).is_err());
        }
    }

    #[test]
    fn dictionary_is_large_and_has_no_duplicates() {
        assert!(WORDS.len() > 100);
        let mut sorted = WORDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), WORDS.len());
    }
}
