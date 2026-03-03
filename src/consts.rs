use std::sync::LazyLock;

use regex::Regex;

pub(crate) static RE_CLEAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[\[\(]?\s*\d{3,5}\s*[x×]?\s*\d{3,5}\s*[\]\)]?|\b\d{6,10}\b)")
        .expect("Failed to compile RE_CLEAN Regex")
});

pub(crate) const DEFAULT_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "heic", "avif",
];

pub(crate) const TRIM_CHARS: &[char] = &[' ', '-', '_', '[', ']', '(', ')'];

#[cfg(test)]
mod re_clean_tests {
    use super::*;

    #[test]
    fn test_re_clean_matches_resolutions() {
        let cases = vec![
            "1920x1080",
            "1920 X 1080",
            "3840×2160", // Unicode multiply sign
            "[1920x1080]",
            "(  1280  x  720  )",
            "  4096  ×  2160  ",
            "[3840 x 2160]",
            "(2560×1440",
            "1920x1080)",
            "Test.1920x1080.BluRay.mkv",
        ];

        for case in cases {
            assert!(RE_CLEAN.is_match(case), "Should match resolution: {}", case);
        }
    }

    #[test]
    fn test_re_clean_matches_digit_runs() {
        // Because `[x×]?` is optional, any run of 6-10 consecutive digits also matches
        // (the second alternative `\b\d{6,10}\b` is partially redundant).
        let cases = vec![
            "123456", // 6 digits
            "1234567",
            "9876543210", // 10 digits
            "0000123456",
            "Movie.12345678.mkv",
            "scene-9876543210-final.jpg",
            "ID_1234567890",
            "photo12345678more", // embedded digit run (no word boundary needed)
            "12345678901",       // 11-digit run still matches a 6-10 substring
        ];

        for case in cases {
            assert!(RE_CLEAN.is_match(case), "Should match digit run: {}", case);
        }
    }

    #[test]
    fn test_re_clean_does_not_match() {
        let cases = vec![
            "1080p", // common but only 4 digits
            "720",
            "4k",
            "12345",     // too short (5 digits)
            "abc123def", // 3 digits
            "version.1.2.3",
            "99x99",          // dimensions too small
            "photo12345more", // 5-digit run
        ];

        for case in cases {
            assert!(!RE_CLEAN.is_match(case), "Should NOT match: {}", case);
        }
    }

    #[test]
    fn test_re_clean_replace_all_cleaning() {
        let test_cases = vec![
            ("Amazing.Scene.1920x1080.1234567890.HDR.mkv", "Amazing.Scene..HDR.mkv"),
            ("Show.S01E01.[3840×2160] ID-9876543210 2024", "Show.S01E01. ID- 2024"),
            (
                "Cool.Movie.(1280 x 720) 5555555555.jpg",
                "Cool.Movie.() 5555555555.jpg", // 5555555555 is 10 digits -> removed
            ),
            ("No.Junk.Here.png", "No.Junk.Here.png"),
            ("123456-video-2560x1440-final-version.webm", "-video--final-version.webm"),
        ];

        for (input, expected) in test_cases {
            let cleaned = RE_CLEAN.replace_all(input, "").to_string();
            assert_eq!(cleaned, expected, "Cleaning failed for: {}", input);
        }
    }

    #[test]
    fn test_re_clean_multiple_matches() {
        let input = "Video 1920x1080 12345678 [3840×2160] 9876543210";
        let cleaned = RE_CLEAN.replace_all(input, "").to_string();
        assert_eq!(cleaned, "Video  ");
    }

    #[test]
    fn test_re_clean_with_default_extensions_context() {
        // Just to document that the regex is often used together with extensions
        // (e.g. only clean image filenames). The regex itself doesn't care about extensions.
        let filename = "photo.1920x1080.1234567890.jpg";
        let cleaned = RE_CLEAN.replace_all(filename, "").to_string();

        // After cleaning you would typically check the extension
        let ext = cleaned.rsplit('.').next().unwrap_or("").to_lowercase();

        assert!(DEFAULT_EXTENSIONS.contains(&ext.as_str()));
        assert_eq!(cleaned, "photo..jpg");
    }
}
