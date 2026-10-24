//! Editing a line of text.

use super::*;

fn typed(text: &str) -> TextInput {
    let mut input = TextInput::default();
    input.insert_str(text);
    input
}

#[test]
fn typing_inserts_at_the_cursor() {
    let mut input = typed("helo");
    input.left();
    input.insert('l');
    assert_eq!(input.text(), "hello");
    input.end();
    input.insert('!');
    assert_eq!(input.text(), "hello!");
}

#[test]
fn editing_works_on_characters_not_bytes() {
    let mut input = typed("日本é");
    input.backspace();
    assert_eq!(input.text(), "日本");
    input.left();
    input.delete();
    assert_eq!(input.text(), "日");
    input.home();
    input.insert('x');
    assert_eq!(input.text(), "x日");
}

#[test]
fn the_cursor_column_counts_wide_characters_twice() {
    let mut input = typed("日本");
    assert_eq!(input.cursor_column(), 4);
    input.left();
    assert_eq!(input.cursor_column(), 2);
}

#[test]
fn words_are_deleted_and_skipped_as_units() {
    let mut input = typed("one two  three");
    input.delete_word_back();
    assert_eq!(input.text(), "one two  ");
    input.delete_word_back();
    assert_eq!(input.text(), "one ");
    input.home();
    input.word_right();
    input.insert('!');
    assert_eq!(input.text(), "one! ");
}

#[test]
fn line_kills_cut_around_the_cursor() {
    let mut input = typed("abcdef");
    input.left();
    input.left();
    input.delete_to_end();
    assert_eq!(input.text(), "abcd");
    input.left();
    input.delete_to_start();
    assert_eq!(input.text(), "d");
}

#[test]
fn pasted_line_breaks_become_spaces() {
    assert_eq!(typed("a\nb\r\nc").text(), "a b  c");
}

#[test]
fn keys_that_are_not_editing_are_left_to_the_caller() {
    let mut input = typed("x");
    let press = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert!(!input.handle_key(press(KeyCode::Enter)));
    assert!(!input.handle_key(press(KeyCode::Esc)));
    assert!(!input.handle_key(press(KeyCode::Tab)));
    assert!(input.handle_key(press(KeyCode::Char('y'))));
    assert!(input.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)));
    assert!(input.is_empty());
}

#[test]
fn take_empties_the_field() {
    let mut input = typed("path");
    assert_eq!(input.take(), "path");
    assert!(input.is_empty());
    assert_eq!(input.cursor_column(), 0);
}
