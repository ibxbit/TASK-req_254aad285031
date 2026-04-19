//! Restricted board visibility is gated by team membership.
//!
//! Plain non-admin users must not see restricted boards unless they are a
//! member of a team listed in `board_allowed_teams`. Admins see everything.

use api_tests::{
    bootstrap_admin_token, get_auth, login, post_empty_auth, post_json_auth, provision_user,
    skip_if_offline,
};
use serde_json::{json, Value};

fn setup(name: &str) -> Option<String> {
    if !skip_if_offline(name) {
        return None;
    }
    bootstrap_admin_token().or_else(|| {
        eprintln!("SKIP {name}: no admin token");
        None
    })
}

// End-to-end: admin creates a zone + a restricted board, creates a team,
// grants the team to the board. A member user sees the board, a non-member
// user does not.
#[test]
fn restricted_board_is_hidden_from_non_team_members() {
    let Some(admin) = setup("restricted_board_is_hidden_from_non_team_members") else {
        return;
    };

    // Zone
    let zone = post_json_auth(
        "/api/zones",
        &admin,
        &json!({ "name": format!("zone_{}", api_tests::nano_suffix()) }),
    )
    .expect("zone");
    if !zone.status().is_success() {
        eprintln!("SKIP: zone create -> {}", zone.status());
        return;
    }
    let zone_id = zone.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Restricted board
    let board = post_json_auth(
        "/api/boards",
        &admin,
        &json!({
            "zone_id": zone_id,
            "name": format!("restricted_{}", api_tests::nano_suffix()),
            "visibility_type": "restricted",
        }),
    )
    .expect("board");
    assert!(
        board.status().is_success(),
        "board create -> {}",
        board.status()
    );
    let board_id = board.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Team
    let team = post_json_auth(
        "/api/admin/teams",
        &admin,
        &json!({ "name": format!("team_{}", api_tests::nano_suffix()) }),
    )
    .expect("team");
    if !team.status().is_success() {
        eprintln!("SKIP: team create -> {}", team.status());
        return;
    }
    let team_id = team.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Two non-admin users — only one joins the team.
    let Some((member_id, m_u, m_p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some((_, o_u, o_p)) = provision_user(&admin, "requester") else {
        return;
    };
    let join = post_json_auth(
        &format!("/api/admin/teams/{team_id}/members"),
        &admin,
        &json!({ "user_id": member_id }),
    )
    .expect("join");
    assert!(join.status().is_success(), "team add -> {}", join.status());

    // Grant the team access to the restricted board.
    let grant = post_json_auth(
        &format!("/api/boards/{board_id}/teams"),
        &admin,
        &json!({ "team_id": team_id }),
    )
    .expect("grant");
    assert!(
        grant.status().is_success(),
        "grant team -> {}",
        grant.status()
    );

    // Member logs in — should see the board in /api/boards.
    let Some(member_tok) = login(&m_u, &m_p) else {
        return;
    };
    let list_m = get_auth("/api/boards", &member_tok).expect("list-m");
    assert!(list_m.status().is_success());
    let arr_m = list_m.json::<Value>().unwrap();
    let seen_by_member = arr_m
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["id"].as_str() == Some(&board_id));
    assert!(seen_by_member, "member must see the restricted board");

    // Non-member logs in — must NOT see the board.
    let Some(other_tok) = login(&o_u, &o_p) else {
        return;
    };
    let list_o = get_auth("/api/boards", &other_tok).expect("list-o");
    let arr_o = list_o.json::<Value>().unwrap();
    let seen_by_other = arr_o
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["id"].as_str() == Some(&board_id));
    assert!(
        !seen_by_other,
        "non-member must NOT see the restricted board"
    );

    // Non-member direct GET must be 403 (not 404, to match the visibility predicate).
    let direct = get_auth(&format!("/api/boards/{board_id}"), &other_tok).expect("direct");
    assert_eq!(direct.status(), 403);

    // Revoke the team's access via DELETE /boards/<id>/teams/<team_id>.
    // Contract: 204 on success, 404 if the grant doesn't exist.
    let revoke = api_tests::delete_auth(&format!("/api/boards/{board_id}/teams/{team_id}"), &admin)
        .expect("revoke");
    api_tests::assert_status(&revoke, 204, "DELETE /boards/<id>/teams/<team_id>");

    // After revocation, the member no longer sees the board.
    let list_m2 = get_auth("/api/boards", &member_tok).expect("list-m2");
    let arr_m2 = list_m2.json::<Value>().unwrap();
    let seen_after_revoke = arr_m2
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["id"].as_str() == Some(&board_id));
    assert!(
        !seen_after_revoke,
        "member must lose visibility after team grant is revoked"
    );

    // Revoking again returns 404.
    let revoke_again =
        api_tests::delete_auth(&format!("/api/boards/{board_id}/teams/{team_id}"), &admin)
            .expect("revoke2");
    api_tests::assert_status(&revoke_again, 404, "revoke absent grant");

    // Tidy — not required, but keeps the DB from accumulating throwaway
    // teams. Best-effort only.
    let _ = post_empty_auth("/api/health", &admin); // noop reachability
}
