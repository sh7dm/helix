use helix_core::{
    graphemes,
    indent::TAB_WIDTH,
    regex::Regex,
    register, selection,
    state::{Direction, Granularity, State},
    ChangeSet, Range, Selection, Tendril, Transaction,
};
use once_cell::sync::Lazy;

use crate::{
    document::Mode,
    prompt::Prompt,
    view::{View, PADDING},
};

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(view: &mut View, count: usize);

pub fn move_char_left(view: &mut View, count: usize) {
    let selection =
        view.doc
            .state
            .move_selection(Direction::Backward, Granularity::Character, count);
    view.doc.set_selection(selection);
}

pub fn move_char_right(view: &mut View, count: usize) {
    let selection =
        view.doc
            .state
            .move_selection(Direction::Forward, Granularity::Character, count);
    view.doc.set_selection(selection);
}

pub fn move_line_up(view: &mut View, count: usize) {
    let selection = view
        .doc
        .state
        .move_selection(Direction::Backward, Granularity::Line, count);
    view.doc.set_selection(selection);
}

pub fn move_line_down(view: &mut View, count: usize) {
    let selection = view
        .doc
        .state
        .move_selection(Direction::Forward, Granularity::Line, count);
    view.doc.set_selection(selection);
}

pub fn move_line_end(view: &mut View, _count: usize) {
    let lines = selection_lines(&view.doc.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.

            // Line end is pos at the start of next line - 1
            // subtract another 1 because the line ends with \n
            view.doc.text().line_to_char(index + 1).saturating_sub(2)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    view.doc.set_selection(selection);
}

pub fn move_line_start(view: &mut View, _count: usize) {
    let lines = selection_lines(&view.doc.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            view.doc.text().line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    view.doc.set_selection(selection);
}

pub fn move_next_word_start(view: &mut View, count: usize) {
    let pos = view.doc.state.move_pos(
        view.doc.selection().cursor(),
        Direction::Forward,
        Granularity::Word,
        count,
    );

    view.doc.set_selection(Selection::point(pos));
}

pub fn move_prev_word_start(view: &mut View, count: usize) {
    let pos = view.doc.state.move_pos(
        view.doc.selection().cursor(),
        Direction::Backward,
        Granularity::Word,
        count,
    );

    view.doc.set_selection(Selection::point(pos));
}

pub fn move_next_word_end(view: &mut View, count: usize) {
    let pos = State::move_next_word_end(
        &view.doc.text().slice(..),
        view.doc.selection().cursor(),
        count,
    );

    view.doc.set_selection(Selection::point(pos));
}

pub fn move_file_start(view: &mut View, _count: usize) {
    view.doc.set_selection(Selection::point(0));

    view.doc.mode = Mode::Normal;
}

pub fn move_file_end(view: &mut View, _count: usize) {
    let text = &view.doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    view.doc.set_selection(Selection::point(last_line));

    view.doc.mode = Mode::Normal;
}

pub fn check_cursor_in_view(view: &mut View) -> bool {
    let cursor = view.doc.selection().cursor();
    let line = view.doc.text().char_to_line(cursor);
    let document_end = view.first_line + view.size.1.saturating_sub(1) as usize;

    if (line > document_end.saturating_sub(PADDING)) | (line < view.first_line + PADDING) {
        return false;
    }
    true
}

pub fn page_up(view: &mut View, _count: usize) {
    if view.first_line < PADDING {
        return;
    }

    view.first_line = view.first_line.saturating_sub(view.size.1 as usize);

    if !check_cursor_in_view(view) {
        let text = view.doc.text();
        let pos = text.line_to_char(view.last_line().saturating_sub(PADDING));
        view.doc.set_selection(Selection::point(pos));
    }
}

pub fn page_down(view: &mut View, _count: usize) {
    view.first_line += view.size.1 as usize + PADDING;

    if view.first_line < view.doc.text().len_lines() {
        let text = view.doc.text();
        let pos = text.line_to_char(view.first_line as usize);
        view.doc.set_selection(Selection::point(pos));
    }
}

pub fn half_page_up(view: &mut View, _count: usize) {
    if view.first_line < PADDING {
        return;
    }

    view.first_line = view.first_line.saturating_sub(view.size.1 as usize / 2);

    if !check_cursor_in_view(view) {
        let text = &view.doc.text();
        let pos = text.line_to_char(view.last_line() - PADDING);
        view.doc.set_selection(Selection::point(pos));
    }
}

pub fn half_page_down(view: &mut View, _count: usize) {
    let lines = view.doc.text().len_lines();
    if view.first_line < lines.saturating_sub(view.size.1 as usize) {
        view.first_line += view.size.1 as usize / 2;
    }
    if !check_cursor_in_view(view) {
        let text = view.doc.text();
        let pos = text.line_to_char(view.first_line as usize);
        view.doc.set_selection(Selection::point(pos));
    }
}
// avoid select by default by having a visual mode switch that makes movements into selects

pub fn extend_char_left(view: &mut View, count: usize) {
    let selection =
        view.doc
            .state
            .extend_selection(Direction::Backward, Granularity::Character, count);
    view.doc.set_selection(selection);
}

pub fn extend_char_right(view: &mut View, count: usize) {
    let selection =
        view.doc
            .state
            .extend_selection(Direction::Forward, Granularity::Character, count);
    view.doc.set_selection(selection);
}

pub fn extend_line_up(view: &mut View, count: usize) {
    let selection = view
        .doc
        .state
        .extend_selection(Direction::Backward, Granularity::Line, count);
    view.doc.set_selection(selection);
}

pub fn extend_line_down(view: &mut View, count: usize) {
    let selection = view
        .doc
        .state
        .extend_selection(Direction::Forward, Granularity::Line, count);
    view.doc.set_selection(selection);
}

pub fn split_selection_on_newline(view: &mut View, _count: usize) {
    let text = &view.doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
    let selection = selection::split_on_matches(text, view.doc.selection(), &REGEX);
    view.doc.set_selection(selection);
}

pub fn select_line(view: &mut View, _count: usize) {
    // TODO: count
    let pos = view.doc.selection().primary();
    let text = view.doc.text();
    let line = text.char_to_line(pos.head);
    let start = text.line_to_char(line);
    let end = text.line_to_char(line + 1).saturating_sub(1);

    view.doc.set_selection(Selection::single(start, end));
}

pub fn delete_selection(view: &mut View, _count: usize) {
    let transaction = Transaction::change_by_selection(&view.doc.state, |range| {
        (range.from(), range.to() + 1, None)
    });
    view.doc.apply(&transaction);

    append_changes_to_history(view);
}

pub fn change_selection(view: &mut View, count: usize) {
    delete_selection(view, count);
    insert_mode(view, count);
}

pub fn collapse_selection(view: &mut View, _count: usize) {
    let selection = view
        .doc
        .selection()
        .transform(|range| Range::new(range.head, range.head));

    view.doc.set_selection(selection);
}

pub fn flip_selections(view: &mut View, _count: usize) {
    let selection = view
        .doc
        .selection()
        .transform(|range| Range::new(range.head, range.anchor));

    view.doc.set_selection(selection);
}

fn enter_insert_mode(view: &mut View) {
    view.doc.mode = Mode::Insert;

    append_changes_to_history(view);
}
// inserts at the start of each selection
pub fn insert_mode(view: &mut View, _count: usize) {
    enter_insert_mode(view);

    let selection = view
        .doc
        .selection()
        .transform(|range| Range::new(range.to(), range.from()));
    view.doc.set_selection(selection);
}

// inserts at the end of each selection
pub fn append_mode(view: &mut View, _count: usize) {
    enter_insert_mode(view);
    view.doc.restore_cursor = true;

    // TODO: as transaction
    let text = &view.doc.text().slice(..);
    let selection = view.doc.selection().transform(|range| {
        // TODO: to() + next char
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()),
        )
    });
    view.doc.set_selection(selection);
}

// TODO: I, A, o and O can share a lot of the primitives.

pub fn command_mode(_view: &mut View, _count: usize) {
    unimplemented!()
}

// calculate line numbers for each selection range
fn selection_lines(state: &State) -> Vec<usize> {
    let mut lines = state
        .selection
        .ranges()
        .iter()
        .map(|range| state.doc.char_to_line(range.head))
        .collect::<Vec<_>>();

    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();

    lines
}

// I inserts at the start of each line with a selection
pub fn prepend_to_line(view: &mut View, count: usize) {
    enter_insert_mode(view);

    move_line_start(view, count);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(view: &mut View, count: usize) {
    enter_insert_mode(view);

    move_line_end(view, count);
}

// o inserts a new line after each line with a selection
pub fn open_below(view: &mut View, _count: usize) {
    enter_insert_mode(view);

    let lines = selection_lines(&view.doc.state);

    let positions: Vec<_> = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line/start of the next one.
            view.doc.text().line_to_char(index + 1)
        })
        .collect();

    // TODO: use same logic as insert_newline for indentation
    let changes = positions.iter().copied().map(|index|
        // generate changes
        (index, index, Some(Tendril::from_char('\n'))));

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(
        positions
            .iter()
            .copied()
            .map(|pos| Range::new(pos, pos))
            .collect(),
        0,
    );

    let transaction = Transaction::change(&view.doc.state, changes).with_selection(selection);

    view.doc.apply(&transaction);
}

// O inserts a new line before each line with a selection

fn append_changes_to_history(view: &mut View) {
    if view.doc.changes.is_empty() {
        return;
    }

    let new_changeset = ChangeSet::new(view.doc.text());
    let changes = std::mem::replace(&mut view.doc.changes, new_changeset);
    // Instead of doing this messy merge we could always commit, and based on transaction
    // annotations either add a new layer or compose into the previous one.
    let transaction = Transaction::from(changes).with_selection(view.doc.selection().clone());

    // increment document version
    // TODO: needs to happen on undo/redo too
    view.doc.version += 1;

    // TODO: trigger lsp/documentDidChange with changes

    // HAXX: we need to reconstruct the state as it was before the changes..
    let old_state = std::mem::replace(&mut view.doc.old_state, view.doc.state.clone());
    // TODO: take transaction by value?
    view.doc.history.commit_revision(&transaction, &old_state);
}

pub fn normal_mode(view: &mut View, _count: usize) {
    view.doc.mode = Mode::Normal;

    append_changes_to_history(view);

    // if leaving append mode, move cursor back by 1
    if view.doc.restore_cursor {
        let text = &view.doc.text().slice(..);
        let selection = view.doc.selection().transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });
        view.doc.set_selection(selection);

        view.doc.restore_cursor = false;
    }
}

pub fn goto_mode(view: &mut View, _count: usize) {
    view.doc.mode = Mode::Goto;
}

// NOTE: Transactions in this module get appended to history when we switch back to normal mode.
pub mod insert {
    use super::*;
    // TODO: insert means add text just before cursor, on exit we should be on the last letter.
    pub fn insert_char(view: &mut View, c: char) {
        let c = Tendril::from_char(c);
        let transaction = Transaction::insert(&view.doc.state, c);

        view.doc.apply(&transaction);
    }

    pub fn insert_tab(view: &mut View, _count: usize) {
        insert_char(view, '\t');
    }

    pub fn insert_newline(view: &mut View, _count: usize) {
        let transaction = Transaction::change_by_selection(&view.doc.state, |range| {
            let indent_level = helix_core::indent::suggested_indent_for_pos(
                view.doc.syntax.as_ref(),
                &view.doc.state,
                range.head,
            );
            let indent = " ".repeat(TAB_WIDTH).repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            (range.head, range.head, Some(text.into()))
        });
        view.doc.apply(&transaction);
    }

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(view: &mut View, count: usize) {
        let text = &view.doc.text().slice(..);
        let transaction = Transaction::change_by_selection(&view.doc.state, |range| {
            (
                graphemes::nth_prev_grapheme_boundary(text, range.head, count),
                range.head,
                None,
            )
        });
        view.doc.apply(&transaction);
    }

    pub fn delete_char_forward(view: &mut View, count: usize) {
        let text = &view.doc.text().slice(..);
        let transaction = Transaction::change_by_selection(&view.doc.state, |range| {
            (
                range.head,
                graphemes::nth_next_grapheme_boundary(text, range.head, count),
                None,
            )
        });
        view.doc.apply(&transaction);
    }
}

pub fn insert_char_prompt(prompt: &mut Prompt, c: char) {
    prompt.insert_char(c);
}

// Undo / Redo

pub fn undo(view: &mut View, _count: usize) {
    if let Some(revert) = view.doc.history.undo() {
        view.doc.version += 1;
        view.doc.apply(&revert);
    }

    // TODO: each command could simply return a Option<transaction>, then the higher level handles storing it?
}

pub fn redo(view: &mut View, _count: usize) {
    if let Some(transaction) = view.doc.history.redo() {
        view.doc.version += 1;
        view.doc.apply(&transaction);
    }
}

// Yank / Paste

pub fn yank(view: &mut View, _count: usize) {
    // TODO: should selections be made end inclusive?
    let values = view
        .doc
        .state
        .selection()
        .fragments(&view.doc.text().slice(..))
        .map(|cow| cow.into_owned())
        .collect();

    // TODO: allow specifying reg
    let reg = '"';
    register::set(reg, values);
}

pub fn paste(view: &mut View, _count: usize) {
    // TODO: allow specifying reg
    let reg = '"';
    if let Some(values) = register::get(reg) {
        let repeat = std::iter::repeat(
            values
                .last()
                .map(|value| Tendril::from_slice(value))
                .unwrap(),
        );

        // TODO: if any of values ends \n it's linewise paste
        //
        // p => paste after
        // P => paste before
        // alt-p => paste every yanked selection after selected text
        // alt-P => paste every yanked selection before selected text
        // R => replace selected text with yanked text
        // alt-R => replace selected text with every yanked text
        //
        // append => insert at next line
        // insert => insert at start of line
        // replace => replace
        // default insert

        let linewise = values.iter().any(|value| value.ends_with('\n'));

        let mut values = values.into_iter().map(Tendril::from).chain(repeat);

        let transaction = if linewise {
            // paste on the next line
            // TODO: can simply take a range + modifier and compute the right pos without ifs
            let text = view.doc.text();
            Transaction::change_by_selection(&view.doc.state, |range| {
                let line_end = text.line_to_char(text.char_to_line(range.head) + 1);
                (line_end, line_end, Some(values.next().unwrap()))
            })
        } else {
            Transaction::change_by_selection(&view.doc.state, |range| {
                (range.head + 1, range.head + 1, Some(values.next().unwrap()))
            })
        };

        view.doc.apply(&transaction);
        append_changes_to_history(view);
    }
}

fn get_lines(view: &View) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in view.doc.selection().ranges() {
        let start = view.doc.text().char_to_line(range.from());
        let end = view.doc.text().char_to_line(range.to());

        for line in start..=end {
            lines.push(line)
        }
    }
    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();
    lines
}

pub fn indent(view: &mut View, _count: usize) {
    let lines = get_lines(view);

    // Indent by one level
    let indent = Tendril::from(" ".repeat(TAB_WIDTH));

    let transaction = Transaction::change(
        &view.doc.state,
        lines.into_iter().map(|line| {
            let pos = view.doc.text().line_to_char(line);
            (pos, pos, Some(indent.clone()))
        }),
    );
    view.doc.apply(&transaction);
    append_changes_to_history(view);
}

pub fn unindent(view: &mut View, _count: usize) {
    let lines = get_lines(view);
    let mut changes = Vec::with_capacity(lines.len());

    for line_idx in lines {
        let line = view.doc.text().line(line_idx);
        let mut width = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / TAB_WIDTH + 1) * TAB_WIDTH,
                _ => break,
            }

            if width >= TAB_WIDTH {
                break;
            }
        }

        if width > 0 {
            let start = view.doc.text().line_to_char(line_idx);
            changes.push((start, start + width, None))
        }
    }

    let transaction = Transaction::change(&view.doc.state, changes.into_iter());

    view.doc.apply(&transaction);
    append_changes_to_history(view);
}

pub fn indent_selection(_view: &mut View, _count: usize) {
    // loop over each line and recompute proper indentation
    unimplemented!()
}
