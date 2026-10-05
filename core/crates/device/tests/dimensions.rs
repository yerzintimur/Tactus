//! The names behind a parameter's repeat dimensions, pinned to the index tables
//! in Roland's V31 MIDI Implementation ("Pad assignments": KitUnitCommon /
//! KitUnitLayer 1–28, KitPad 1–14, KitFx 1–8) — the data a panel edit on the
//! snare rim is named from.

use device::{DimLabel, ProfileRegistry};

fn v31() -> device::DeviceProfile {
    ProfileRegistry::with_builtin()
        .match_model(&[1, 6, 1])
        .expect("built-in V31 profile")
        .clone()
}

/// Every dimension a parameter repeats over can name all of its positions:
/// either one label per position, or a numbered phrase. A position the profile
/// cannot name would otherwise be spoken as nothing at all.
#[test]
fn every_dimension_names_every_position() {
    for p in ProfileRegistry::with_builtin().profiles() {
        for param in &p.parameters {
            for dim in &param.dims {
                let def = p.dimensions.get(&dim.name).unwrap_or_else(|| {
                    panic!(
                        "{}: {}: dimension {:?} has no names",
                        p.profile_id, param.id, dim.name
                    )
                });
                if def.i18n_key.is_none() {
                    assert_eq!(
                        def.labels.len() as u32,
                        dim.count,
                        "{}: {}: dimension {:?} names {} of {} positions",
                        p.profile_id,
                        param.id,
                        dim.name,
                        def.labels.len(),
                        dim.count
                    );
                }
                assert!(p.dim_label(dim, 0).is_some());
                assert!(p.dim_label(dim, dim.count - 1).is_some());
            }
        }
    }
}

/// The unit order as the MIDI Implementation lists it — and as the module sent
/// it: the snare's layer-A edit arrived for units 1 and 2 (head, rim).
#[test]
fn units_follow_the_modules_index_table() {
    let p = v31();
    let volume = p.parameter("kit.unit.layer.volume").unwrap();
    let unit = volume.dims.iter().find(|d| d.name == "unit").unwrap();
    let at = |i| p.dim_label(unit, i);
    assert_eq!(at(0), Some(DimLabel::Named("pad.kick")));
    assert_eq!(at(1), Some(DimLabel::Named("pad.snare.head")));
    assert_eq!(at(2), Some(DimLabel::Named("pad.snare.rim")));
    assert_eq!(at(11), Some(DimLabel::Named("pad.hihat.head")));
    assert_eq!(at(19), Some(DimLabel::Named("pad.ride.bell")));
    // The module's first aux input is plain AUX; AUX2–4 follow.
    assert_eq!(at(20), Some(DimLabel::Named("pad.aux.head")));
    assert_eq!(at(27), Some(DimLabel::Named("pad.aux4.rim")));
    assert_eq!(at(28), None, "28 units, no more");
}

#[test]
fn pads_layers_and_fx_slots_are_named() {
    let p = v31();
    let pan = p.parameter("kit.pad.pan").unwrap();
    assert_eq!(
        p.dim_label(&pan.dims[0], 13),
        Some(DimLabel::Named("pad.aux4"))
    );

    let volume = p.parameter("kit.unit.layer.volume").unwrap();
    let layer = volume.dims.iter().find(|d| d.name == "layer").unwrap();
    assert_eq!(p.dim_label(layer, 2), Some(DimLabel::Named("layer.c")));

    // KitFx 1–8 = BUS-A FX1, BUS-A FX2, …, BUS-D FX2.
    let fx = p.parameter("kit.fx.type").unwrap();
    assert_eq!(
        p.dim_label(&fx.dims[0], 0),
        Some(DimLabel::Named("fx.bus_a.1"))
    );
    assert_eq!(
        p.dim_label(&fx.dims[0], 7),
        Some(DimLabel::Named("fx.bus_d.2"))
    );
}

/// Set-list steps have no names: the profile hands out a numbered phrase with
/// the 1-based number the module's screen shows.
#[test]
fn unnamed_positions_are_numbered_from_one() {
    let p = v31();
    let step = p.parameter("setlist.step").unwrap();
    assert_eq!(
        p.dim_label(&step.dims[0], 4),
        Some(DimLabel::Numbered {
            key: "dim.step",
            number: 5
        })
    );
}

/// `Located` splits the indices `locate` found into the area's and the dims'.
#[test]
fn located_separates_the_dim_indices_from_the_kit() {
    let p = v31();
    // The snare rim's layer-A volume on kit 10, as the module sent it.
    let located = p.locate([0x04, 0x24, 0x54, 0x09]).unwrap();
    assert_eq!(located.indices, vec![9, 0, 2]);
    assert_eq!(located.dim_indices(), &[0, 2]);
    // No area index on `current`, no dims either.
    let current = p.locate([0, 0, 0, 0]).unwrap();
    assert_eq!(current.dim_indices(), &[] as &[u32]);
}
