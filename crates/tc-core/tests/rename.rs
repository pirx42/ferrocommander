//! The multi-rename rules, table-driven as the design doc asks.

use tc_core::rename::{preview, Refusal, Renamed, Rules, COUNTER, EXT, NAME};

/// Rules with a template and nothing else changed.
fn with(template: &str) -> Rules {
    Rules {
        template: template.to_string(),
        ..Rules::default()
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| name.to_string()).collect()
}

/// Just the produced names, for the table below.
fn produced(rules: &Rules, from: &[&str]) -> Vec<String> {
    preview(rules, &names(from))
        .into_iter()
        .map(|row| row.to)
        .collect()
}

#[test]
fn the_rules_table() {
    // rules × names × expected. One row per thing the template language can
    // do, and per way it can be combined.
    let cases: &[(Rules, &[&str], &[&str])] = &[
        // The default renames nothing at all: a tool that changed something
        // before the user typed anything would be a trap.
        (Rules::default(), &["a.txt", "b.md"], &["a.txt", "b.md"]),
        // The placeholders on their own.
        (with(NAME), &["report.txt"], &["report"]),
        (with(EXT), &["report.txt"], &["txt"]),
        (with(COUNTER), &["a", "b", "c"], &["1", "2", "3"]),
        // And in combination, which is what anybody actually types.
        (
            with(&format!("photo-{COUNTER}.{EXT}")),
            &["DSC1.jpg", "DSC2.jpg"],
            &["photo-1.jpg", "photo-2.jpg"],
        ),
        (
            with(&format!("{NAME}-backup.{EXT}")),
            &["notes.txt"],
            &["notes-backup.txt"],
        ),
        // Literal text with no placeholder in it is used as it stands, which
        // is what makes the collision rule below matter.
        (with("fixed"), &["one.txt"], &["fixed"]),
        // A file with no extension leaves no trailing dot behind.
        (with(&format!("{NAME}.{EXT}")), &["README"], &["README"]),
        (with(&format!("{NAME}.{EXT}")), &["a.b.c"], &["a.b.c"]),
        // A dot in the middle of a name is not the extension separator.
        (with(NAME), &["archive.tar.gz"], &["archive.tar"]),
        (with(EXT), &["archive.tar.gz"], &["gz"]),
    ];

    for (rules, from, expected) in cases {
        assert_eq!(
            produced(rules, from),
            names(expected),
            "template {:?} on {from:?}",
            rules.template
        );
    }
}

#[test]
fn the_counter_starts_where_it_is_told() {
    let rules = Rules {
        template: format!("{COUNTER}.{EXT}"),
        counter_start: 100,
        ..Rules::default()
    };

    assert_eq!(
        produced(&rules, &["a.txt", "b.txt"]),
        names(&["100.txt", "101.txt"])
    );
}

#[test]
fn search_and_replace_runs_over_the_result() {
    // Over the result, not the original: it is the last thing applied, so it
    // can fix up whatever the template produced.
    let rules = Rules {
        template: format!("{NAME}.{EXT}"),
        find: "draft".to_string(),
        replace: "final".to_string(),
        ..Rules::default()
    };

    assert_eq!(
        produced(&rules, &["draft-one.txt", "two-draft.txt", "other.txt"]),
        names(&["final-one.txt", "two-final.txt", "other.txt"])
    );
}

#[test]
fn an_empty_replacement_removes_what_it_finds() {
    let rules = Rules {
        template: NAME.to_string(),
        find: "_old".to_string(),
        replace: String::new(),
        ..Rules::default()
    };

    assert_eq!(produced(&rules, &["notes_old"]), names(&["notes"]));
}

#[test]
fn a_preview_that_produces_nothing_refuses_rather_than_renaming() {
    // A file with no name is not a file.
    let rules = Rules {
        template: NAME.to_string(),
        find: "everything".to_string(),
        replace: String::new(),
        ..Rules::default()
    };

    let rows = preview(&rules, &names(&["everything"]));

    assert_eq!(rows[0].refused, Some(Refusal::Empty));
    assert!(!rows[0].changes_anything());
}

#[test]
fn two_files_cannot_be_given_the_same_name() {
    // The invariant that makes this tool safe to point at a hundred files: a
    // batch never produces two identical names, so no rename in it can
    // silently eat another file.
    let rows = preview(&with("same.txt"), &names(&["a.txt", "b.txt", "c.txt"]));

    assert_eq!(rows[0].refused, None, "the first may have it");
    assert_eq!(rows[1].refused, Some(Refusal::Collides));
    assert_eq!(rows[2].refused, Some(Refusal::Collides));
}

#[test]
fn a_name_with_a_separator_in_it_is_refused() {
    // That is a move, and this tool renames. Letting it through would turn a
    // typo in a template into files scattered across the filesystem.
    for template in ["../escaped", "sub/dir", "back\\slash"] {
        let rows = preview(&with(template), &names(&["a.txt"]));
        assert_eq!(
            rows[0].refused,
            Some(Refusal::HasSeparator),
            "template {template:?}"
        );
    }
}

#[test]
fn every_input_gets_a_row_whatever_happens_to_it() {
    // A preview that silently dropped the files it would not touch would be a
    // preview of something other than what is about to happen.
    let from = names(&["a.txt", "b.txt", "c.txt"]);

    let rows = preview(&with("same"), &from);

    assert_eq!(rows.len(), from.len());
    assert_eq!(
        rows.iter().map(|row| row.from.clone()).collect::<Vec<_>>(),
        from
    );
}

#[test]
fn previewing_twice_gives_the_same_answer() {
    // It is a pure function and the tool leans on that: the preview a person
    // reads and the rename that runs are the same call.
    let rules = Rules {
        template: format!("{COUNTER}-{NAME}.{EXT}"),
        counter_start: 7,
        find: "a".to_string(),
        replace: "z".to_string(),
    };
    let from = names(&["alpha.txt", "beta.md", "gamma"]);

    assert_eq!(preview(&rules, &from), preview(&rules, &from));
}

#[test]
fn a_row_that_does_not_change_the_name_is_not_a_rename() {
    let rows = preview(&Rules::default(), &names(&["unchanged.txt"]));

    assert_eq!(
        rows[0],
        Renamed {
            from: "unchanged.txt".to_string(),
            to: "unchanged.txt".to_string(),
            refused: None,
        }
    );
    assert!(!rows[0].changes_anything());
}
