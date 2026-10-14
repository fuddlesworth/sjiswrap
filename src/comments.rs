use encoding_rs::SHIFT_JIS;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Code,
    LineComment,
    BlockComment,
    StringLiteral,
    CharLiteral,
}

/// Finds the first character that Shift JIS can't encode outside a C or C++ comment, with its 1-based line.
///
/// The compiler never sees a comment's text, so a character in one (such as the é of "Pokémon") is harmless,
/// while one in code or in a literal changes the output.
pub fn find_unencodable_outside_comments(source: &str) -> Option<(usize, char)> {
    let mut state = State::Code;
    let mut line = 1;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match state {
            State::Code => match c {
                '/' if chars.peek() == Some(&'/') => {
                    chars.next();
                    state = State::LineComment;
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    state = State::BlockComment;
                }
                '"' => state = State::StringLiteral,
                '\'' => state = State::CharLiteral,
                _ => {}
            },
            State::LineComment => match c {
                // A backslash at the end of the line continues the comment
                '\\' if chars.peek() == Some(&'\n') => {
                    chars.next();
                    line += 1;
                    continue;
                }
                '\n' => state = State::Code,
                _ => {}
            },
            State::BlockComment => {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    state = State::Code;
                }
            }
            State::StringLiteral | State::CharLiteral => match c {
                '\\' => {
                    if chars.next() == Some('\n') {
                        line += 1;
                    }
                    continue;
                }
                '"' if state == State::StringLiteral => state = State::Code,
                '\'' if state == State::CharLiteral => state = State::Code,
                // An unterminated literal ends at the end of its line
                '\n' => state = State::Code,
                _ => {}
            },
        }
        if c == '\n' {
            line += 1;
        } else if !c.is_ascii()
            && !matches!(state, State::LineComment | State::BlockComment)
            && !is_encodable(c)
        {
            return Some((line, c));
        }
    }
    None
}

fn is_encodable(c: char) -> bool {
    let mut buffer = [0u8; 4];
    let (_, _, error) = SHIFT_JIS.encode(c.encode_utf8(&mut buffer));
    !error
}

#[cfg(test)]
mod tests {
    use super::find_unencodable_outside_comments as find;

    #[test]
    fn ignores_comments() {
        assert_eq!(find("// Pokémon\nint x; /* café\n é */ int y;\n"), None);
    }

    #[test]
    fn finds_literals_and_code() {
        assert_eq!(find("const char *s = \"Pokémon\";\n"), Some((1, 'é')));
        assert_eq!(find("// ok\nint c = 'é';\n"), Some((2, 'é')));
        assert_eq!(find("/* a\n b */\nint é;\n"), Some((3, 'é')));
    }

    #[test]
    fn allows_shift_jis() {
        assert_eq!(find("const char *s = \"非対応のOBJマッピングモード\"; // é\n"), None);
    }

    #[test]
    fn comment_markers_in_literals() {
        assert_eq!(find("const char *s = \"// \\\" é\";\n"), Some((1, 'é')));
        assert_eq!(find("char c = '\"'; // é\n"), None);
    }

    #[test]
    fn continued_line_comment() {
        assert_eq!(find("// a \\\n é\nint x;\n"), None);
        assert_eq!(find("// a\n é\n"), Some((2, 'é')));
    }
}
