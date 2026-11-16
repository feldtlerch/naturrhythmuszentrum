// SPDX-License-Identifier: AGPL-3.0-only
//! Literal outcomes and independently obtained publication receipts check trace evidence.

use super::{Snapshot, TestResult, subscriber::Records};
use crate::{HostChannel, LcInputInventory, LcShapeTile};
use nrz_scalar::Scalar;

/// Verify one actual production attempt against literal costs and original publication locators.
pub(super) fn check<P: Scalar + 'static>(
    records: &Records,
    case: usize,
    quantities: bool,
    snapshot: &Snapshot<P>,
    tiles: &[&LcShapeTile<'_, P>],
    inventory: &LcInputInventory<P, 5>,
) -> TestResult<u64> {
    let required = if quantities { 46 } else { 43 };
    let work = if case >= 8 { 0 } else { required };
    let native = match case {
        0 => [15, if quantities { 5 } else { 3 }],
        1 | 2 | 4 => [15, 0],
        3 | 5 => [15, 1],
        _ => [0, 0],
    };
    assert_eq!(snapshot.work, work);
    assert_eq!(snapshot.native, native);
    assert_eq!(records.events.len(), 7);
    let barrier = records.spans.first().ok_or("barrier")?;
    assert_eq!(barrier.name, "nrz.barrier");
    assert_eq!(barrier.fields["family"], "lc");
    assert_eq!(barrier.fields["question"], "frequency");
    assert_eq!(barrier.fields["operation"], "positive-domain");
    assert_eq!(
        barrier.fields["consumer"],
        if quantities {
            "quantities"
        } else {
            "frequency"
        }
    );
    assert_eq!(
        barrier.fields["schema"],
        inventory
            .input(0)
            .ok_or("schema")?
            .specialization_key()
            .to_string()
    );
    assert_eq!(barrier.fields["axes"], format!("{:?}", tiles[0].axes()));
    assert_eq!(barrier.fields["format"], format!("{:?}", P::PRECISION));
    assert_eq!(barrier.fields["inductance"], "0");
    assert_eq!(barrier.fields["capacitance"], "1");
    assert_eq!(barrier.fields["cell"], if case == 7 { "12" } else { "3" });
    let attempt = barrier.fields["attempt"].parse::<u64>()?;
    assert_ne!(attempt, 0);
    let interval = barrier.work.ok_or("barrier work")?;
    assert_eq!(interval[1] - interval[0], work);
    assert_eq!(records.spans.len(), if case == 8 { 1 } else { 2 });
    if case != 8 {
        let phase = &records.spans[1];
        assert_eq!(phase.name, "nrz.work.phase");
        assert_eq!(phase.parent, Some(1));
        let interval = phase.work.ok_or("phase work")?;
        assert_eq!(interval[1] - interval[0], work);
        assert!(phase.fields.contains_key("ledger"));
    }
    for event in &records.events {
        assert_eq!(event.parent, Some(1));
        assert_eq!(event.fields["attempt"], attempt.to_string());
    }
    let first = &records.events[0];
    assert_eq!(first.name, "nrz.decision");
    assert_eq!(first.fields["outcome"], "attempted");
    assert_eq!(first.fields["work"], interval[0].to_string());
    claims(records, tiles, inventory)?;
    terminal(records, case, snapshot)?;
    Ok(attempt)
}

/// Every current publication, including the unused repeated role, has its exact original schema.
fn claims<P: Scalar + 'static>(
    records: &Records,
    tiles: &[&LcShapeTile<'_, P>],
    inventory: &LcInputInventory<P, 5>,
) -> TestResult {
    for (position, (event, tile)) in records.events[1..6].iter().zip(tiles).enumerate() {
        assert_eq!(event.name, "nrz.claim");
        assert_eq!(event.fields["position"], position.to_string());
        assert_eq!(
            event.fields["schema"],
            inventory
                .input(position)
                .ok_or("original schema")?
                .specialization_key()
                .to_string()
        );
        assert_eq!(
            event.fields["occupation"],
            format!("{:?}", tile.receipt().ticket().identity)
        );
        assert_eq!(
            event.fields["companions"],
            format!(
                "{:?}",
                HostChannel::ALL.map(|channel| tile.receipt().filled(channel))
            )
        );
        assert!(event.fields["source"].starts_with("Some("));
        assert!(event.fields["source_companions"].starts_with("Some("));
        assert_eq!(event.fields["error_scope"], "HostInput");
    }
    Ok(())
}

/// Literal conclusions distinguish domain outcomes from capability, availability and budget refusal.
fn terminal<P: Scalar + 'static>(
    records: &Records,
    case: usize,
    snapshot: &Snapshot<P>,
) -> TestResult {
    let (outcome, target, reason, continuation) = match case {
        0 => ("completed", "none", "positive", "local-ready"),
        1 => (
            "undecided",
            "inductance",
            "no-strict-positive-separation",
            "retain-unresolved-inputs",
        ),
        2 => (
            "undecided",
            "capacitance",
            "no-strict-positive-separation",
            "retain-unresolved-inputs",
        ),
        3 => (
            "undecided",
            "product",
            "no-strict-positive-separation",
            "retain-unresolved-inputs",
        ),
        4 => (
            "invalid",
            "inductance",
            "strict-negative-box",
            "invalid-domain",
        ),
        5 => ("refused", "none", "arithmetic", "none"),
        6 => ("refused", "none", "coverage", "none"),
        7 => ("refused", "none", "capacity-or-layout", "none"),
        8 => ("refused", "none", "foreign-account", "none"),
        _ => ("refused", "none", "budget", "none"),
    };
    let event = &records.events[6];
    assert_eq!(event.name, "nrz.decision");
    for (field, value) in [
        ("outcome", outcome),
        ("target", target),
        ("reason", reason),
        ("continuation", continuation),
    ] {
        assert_eq!(event.fields[field], value);
    }
    assert_eq!(records.spans[0].fields["outcome"], outcome);
    assert_eq!(records.spans[0].fields["reason"], reason);
    assert_eq!(
        event.fields["work"],
        records.spans[0].work.ok_or("final work")?[1].to_string()
    );
    if case < 5 {
        let image = snapshot
            .image
            .as_ref()
            .map_err(|_| "successful sign result")?;
        assert_eq!(
            image.state,
            [
                "Ready",
                "Undecided(Inductance)",
                "Undecided(Capacitance)",
                "Undecided(Product)",
                "Invalid(Inductance)"
            ][case]
        );
        assert_eq!(image.values.is_some(), case == 0);
    } else {
        assert!(snapshot.image.is_err());
    }
    Ok(())
}
