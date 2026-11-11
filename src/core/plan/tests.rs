use super::*;

fn database() -> (Connection, Project, Project) {
    let conn = Connection::open_in_memory().unwrap();
    sqlite::initialize_schema(&conn).unwrap();
    conn.execute_batch(
        "INSERT INTO projects (id, name, directory) VALUES ('p', 'Trail', '/trail');
         INSERT INTO projects (id, name, directory) VALUES ('o', 'Other', '/other');",
    )
    .unwrap();
    let mut projects = sqlite::get_projects(&conn).unwrap();
    projects.sort_by(|a, b| a.id.cmp(&b.id));
    let trail = projects.pop().unwrap();
    let other = projects.pop().unwrap();
    (conn, trail, other)
}

fn add(conn: &Connection, project: &Project, items: &[&str]) -> Vec<i64> {
    let add: Vec<String> = items.iter().map(|item| item.to_string()).collect();
    let edit = PlanEdit {
        add: &add,
        ..PlanEdit::default()
    };
    edit_plan(conn, project, &edit)
        .unwrap()
        .iter()
        .map(|item| item.id)
        .collect()
}

fn texts(items: &[PlanItemInfo]) -> Vec<(&str, PlanState)> {
    items
        .iter()
        .map(|item| (item.text.as_str(), item.state))
        .collect()
}

#[test]
fn one_edit_adds_finishes_and_drops_and_the_open_plan_keeps_its_order() {
    let (conn, trail, _) = database();
    let ids = add(&conn, &trail, &["One", "Two", "Three"]);

    let add = vec!["  Four  ".to_string()];
    let changed = edit_plan(
        &conn,
        &trail,
        &PlanEdit {
            add: &add,
            done: &[ids[0]],
            drop: &[ids[2]],
        },
    )
    .unwrap();

    assert_eq!(
        texts(&changed),
        [
            ("Four", PlanState::Open),
            ("One", PlanState::Done),
            ("Three", PlanState::Dropped),
        ]
    );
    assert!(changed[1].closed_at.is_some());
    assert_eq!(changed[1].closed_by_update, None);
    assert_eq!(
        texts(&plan(&conn, &trail, false).unwrap()),
        [("Two", PlanState::Open), ("Four", PlanState::Open)]
    );
    assert_eq!(plan(&conn, &trail, true).unwrap().len(), 4);
}

#[test]
fn any_bad_item_or_id_rejects_the_whole_edit() {
    let (conn, trail, other) = database();
    let ids = add(&conn, &trail, &["Open", "Finished"]);
    let foreign = add(&conn, &other, &["Theirs"])[0];
    edit_plan(
        &conn,
        &trail,
        &PlanEdit {
            done: &[ids[1]],
            ..PlanEdit::default()
        },
    )
    .unwrap();
    let new_item = vec!["New".to_string()];

    for (add, done, drop, message) in [
        (vec![], vec![], vec![], "nothing to change"),
        (vec![" ".to_string()], vec![], vec![], "can't be empty"),
        (vec!["a\nb".to_string()], vec![], vec![], "one line"),
        (
            new_item.clone(),
            vec![999],
            vec![],
            "there's no plan item 999",
        ),
        (
            new_item.clone(),
            vec![foreign],
            vec![],
            "isn't in Trail's plan",
        ),
        (new_item.clone(), vec![ids[1]], vec![], "is already done"),
        (new_item.clone(), vec![ids[0]], vec![ids[0]], "listed twice"),
    ] {
        let edit = PlanEdit {
            add: &add,
            done: &done,
            drop: &drop,
        };
        let error = edit_plan(&conn, &trail, &edit).err().unwrap().to_string();
        assert!(error.contains(message), "{error}");
    }

    assert_eq!(
        texts(&plan(&conn, &trail, true).unwrap()),
        [("Open", PlanState::Open), ("Finished", PlanState::Done)]
    );
    assert_eq!(plan(&conn, &other, false).unwrap().len(), 1);
}
