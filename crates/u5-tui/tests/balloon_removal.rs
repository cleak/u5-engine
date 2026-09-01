//! Gap #17 - `systems/vehicles.md` sections 2, 3 and 11: there is no balloon
//! vehicle family, so the debug CLI must not be able to seed one.
//!
//! Section 2: "**There is no balloon and no sixth vehicle family.**" ...
//! "Do not model balloon art as a transport state."
//!
//! Section 3 family table, Balloon row: "Vehicle tile family only in the
//! analyzed baseline." / "No command-level balloon mechanics are specified
//! for v1; do not infer a boardable vehicle from art alone."
//!
//! Section 11 "Balloon boundary": "Settled, not merely untraced. Balloon
//! sprites are catalog assets only. No value a balloon could occupy is
//! written or read by any shipped binary, and Section 2 gives the
//! arithmetic argument that closes the last route by which such a value
//! could have been reached. Do not invent boarding, landing, or
//! wind-driven balloon movement."
//!
//! `--transport balloon` was the only reachable producer of the removed
//! `TransportState::Balloon` variant anywhere in the engine, so this is the
//! boundary where the removal is observable.

use u5_runtime::TransportState;
use u5_tui::*;

#[test]
fn transport_arg_rejects_balloon_as_an_unknown_transport() {
    let error = parse_transport_arg("balloon")
        .expect_err("vehicles.md section 11: balloon sprites are catalog assets only");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    let text = error.to_string();
    assert!(
        text.contains("unknown transport"),
        "balloon must fall through to the unknown-transport arm, got: {text}"
    );
    // The message echoes the rejected input, so only the accepted-kind
    // list is checked for the withdrawn family.
    assert!(
        text.contains("expected foot|horse|ship|skiff|carpet"),
        "the accepted-kind list must be the five published families, got: {text}"
    );
    assert!(
        !text.contains("carpet|balloon"),
        "the accepted-kind list must not advertise balloon, got: {text}"
    );
}

#[test]
fn transport_arg_still_accepts_the_five_published_vehicle_families() {
    // vehicles.md section 3 family table: Foot, Horse, Ship, Skiff, Magic
    // carpet. Removing the balloon must not disturb any of them.
    assert!(matches!(
        parse_transport_arg("foot").unwrap(),
        TransportState::Foot
    ));
    assert!(matches!(
        parse_transport_arg("horse").unwrap(),
        TransportState::Horse { .. }
    ));
    assert!(matches!(
        parse_transport_arg("ship").unwrap(),
        TransportState::Ship { .. }
    ));
    assert!(matches!(
        parse_transport_arg("skiff").unwrap(),
        TransportState::Skiff { .. }
    ));
    assert!(matches!(
        parse_transport_arg("carpet").unwrap(),
        TransportState::Carpet { .. }
    ));
}

#[test]
fn transport_usage_line_lists_no_balloon() {
    // vehicles.md section 11: do not advertise a balloon transport path.
    let usage = CLI_USAGE;
    assert!(
        usage.contains("--transport"),
        "usage text must still document --transport"
    );
    assert!(
        !usage.to_ascii_lowercase().contains("balloon"),
        "usage text must not offer a balloon transport"
    );
}
