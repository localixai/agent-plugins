use super::*;

fn word(value: &str) -> ShellWord {
    ShellWord {
        text: value.to_owned(),
        start: 0.0,
        end: 0.0,
        operator: false,
        value: value.to_owned(),
        expands: false,
        globs: false,
    }
}

fn operands(words: &[&str]) -> Option<Vec<String>> {
    let words: Vec<ShellWord> = words.iter().map(|value| word(value)).collect();
    sed_operands(&words).map(|found| found.into_iter().map(|word| word.value).collect())
}

#[test]
fn the_patterns_compile() {
    assert!(PLAIN_OPTION.is_some());
    assert!(EXPRESSION.is_some());
}

// bun -e 'import { sedOperands } from "./src/rules/decide/sed-script.ts";
//   const w = (value) => ({ text: value, start: 0, end: 0, operator: false, value, expands: false, globs: false });
//   for (const s of ["s\u{1F600}a\u{1F600}b\u{1F600}", "\\\u{1F600}x\u{1F600}p", "y\u{1F600}ab\u{1F600}cd\u{1F600}",
//     "s/a/b/\u{2028}p", "p\u{85}", "1~2p;\u{feff}d"])
//     console.log(JSON.stringify(sedOperands([w(s), w("f")])?.map((x) => x.value)));
//   console.log(JSON.stringify(sedOperands([w("-i x"), w("p"), w("f")])?.map((x) => x.value)),
//     JSON.stringify(sedOperands([w("-e p"), w("f")])?.map((x) => x.value)))'
// prints undefined five times, then ["f"], then undefined and ["f"]: the script is read by UTF-16
// code unit, so a delimiter above U+FFFF leaves its low surrogate behind; `\s` is JavaScript's;
// `.` stops at U+2028 without the `s` flag.
#[test]
fn the_script_is_read_as_javascript_reads_it() {
    let unsafe_scripts = [
        "s\u{1F600}a\u{1F600}b\u{1F600}",
        "\\\u{1F600}x\u{1F600}p",
        "y\u{1F600}ab\u{1F600}cd\u{1F600}",
        "s/a/b/\u{2028}p",
        "p\u{85}",
    ];
    for script in unsafe_scripts {
        assert_eq!(operands(&[script, "f"]), None, "{script:?}");
    }
    assert_eq!(
        operands(&["1~2p;\u{feff}d", "f"]),
        Some(vec!["f".to_owned()])
    );
    assert_eq!(operands(&["-i\u{2028}x", "p", "f"]), None);
    assert_eq!(operands(&["-e\u{2028}p", "f"]), Some(vec!["f".to_owned()]));
}
