use tournament_manager_rs::{field_control::FieldQueue, model::*, scoring::Game};
fn event() -> Event {
    let mut e = Event::default();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    e.matches = (1..=3)
        .map(|id| g.new_match(id, 2, &[1, 2, 3, 4]))
        .collect();
    e
}
#[test]
fn viewing_and_queueing_do_not_change_active_match_or_scores() {
    let e = event();
    let mut q = FieldQueue::default();
    q.enqueue(&e, 2, Some(1)).unwrap();
    q.enqueue(&e, 3, Some(1)).unwrap();
    q.enqueue(&e, 2, Some(1)).unwrap();
    assert_eq!(q.pending, vec![2, 3]);
    assert!(e.matches.iter().all(|m| m.state == MatchState::Unplayed));
    assert!(q.enqueue(&e, 1, Some(1)).is_err());
    assert_eq!(q.next(&e, false).unwrap(), 2);
    assert!(q.next(&e, true).is_err());
}
#[test]
fn queue_skips_submitted_or_deleted_matches_and_removes_loaded_match() {
    let mut e = event();
    let mut q = FieldQueue::default();
    q.enqueue(&e, 1, None).unwrap();
    q.enqueue(&e, 2, None).unwrap();
    e.matches[0].state = MatchState::Submitted;
    assert_eq!(q.next(&e, false).unwrap(), 2);
    assert!(q.enqueue(&e, 1, None).is_err());
    q.loaded(&e, 2);
    assert!(q.pending.is_empty());
    assert!(q.next(&e, false).is_err());
}
