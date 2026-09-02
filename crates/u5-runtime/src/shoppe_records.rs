//! Per-shop SHOPPE.DAT record selection helpers per
//! `formats/shoppe-dat.md` §6 and `systems/shops.md` §4.
//!
//! Each shop kind reads from a published cluster of records inside
//! the shared SHOPPE.DAT file. Most clusters are one contiguous run,
//! tiling from record 0 upward (shared barks 0-7), but two are not:
//! the sage records are interleaved inside the tavern band (`84-88`
//! and `91`, with `89`/`90` belonging to tavern branches) and the
//! healer cluster skips `164` (`163` and `165-173`). Every band is
//! therefore modelled as a list of inclusive sub-runs rather than a
//! single first/last pair, so a cluster can never silently claim
//! records it does not own.

use crate::shops::*;

/// Identifies which SHOPPE.DAT band a record id falls into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShoppeBand {
    SharedBark,
    ArmsDescription,
    ArmsSell,
    Tavern,
    Sage,
    HorseTrader,
    ShipBroker,
    Reagent,
    Guild,
    Healer,
    Innkeeper,
}

impl ShoppeBand {
    /// `formats/shoppe-dat.md §6`: the inclusive `(first, last)`
    /// sub-runs this band owns, in ascending order. Most bands are
    /// one run; the sage and healer clusters are published with holes
    /// and own two runs each.
    pub const fn bands(self) -> &'static [(usize, usize)] {
        match self {
            Self::SharedBark => &[(
                SHOPPE_RECORDS_SHARED_BARKS_FIRST,
                SHOPPE_RECORDS_SHARED_BARKS_LAST,
            )],
            Self::ArmsDescription => &[(
                SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST,
                SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST,
            )],
            Self::ArmsSell => &[(
                SHOPPE_RECORDS_ARMS_SELL_FIRST,
                SHOPPE_RECORDS_ARMS_SELL_LAST,
            )],
            Self::Tavern => &[(SHOPPE_RECORDS_TAVERN_FIRST, SHOPPE_RECORDS_TAVERN_LAST)],
            Self::Sage => &SHOPPE_RECORDS_SAGE_BANDS,
            Self::HorseTrader => &[(
                SHOPPE_RECORDS_HORSE_TRADER_FIRST,
                SHOPPE_RECORDS_HORSE_TRADER_LAST,
            )],
            Self::ShipBroker => &[(
                SHOPPE_RECORDS_SHIP_BROKER_FIRST,
                SHOPPE_RECORDS_SHIP_BROKER_LAST,
            )],
            Self::Reagent => &[(SHOPPE_RECORDS_REAGENT_FIRST, SHOPPE_RECORDS_REAGENT_LAST)],
            Self::Guild => &[(SHOPPE_RECORDS_GUILD_FIRST, SHOPPE_RECORDS_GUILD_LAST)],
            Self::Healer => &SHOPPE_RECORDS_HEALER_BANDS,
            Self::Innkeeper => &[(
                SHOPPE_RECORDS_INNKEEPER_FIRST,
                SHOPPE_RECORDS_INNKEEPER_LAST,
            )],
        }
    }

    /// Lowest record id this band owns.
    pub const fn first(self) -> usize {
        self.bands()[0].0
    }

    /// Highest record id this band owns.
    pub const fn last(self) -> usize {
        let bands = self.bands();
        bands[bands.len() - 1].1
    }

    /// Number of record slots this band owns, summed over its
    /// sub-runs. Holes between sub-runs are not counted.
    pub const fn len(self) -> usize {
        let bands = self.bands();
        let mut total = 0;
        let mut index = 0;
        while index < bands.len() {
            let (first, last) = bands[index];
            total += last - first + 1;
            index += 1;
        }
        total
    }

    /// Returns `true` when the record id lies within this band.
    pub const fn contains(self, id: usize) -> bool {
        shoppe_record_in_bands(id, self.bands())
    }

    /// Classify a record id into its band, picking the most specific
    /// match when bands overlap (sage records sit inside the tavern
    /// band).
    pub fn classify(id: usize) -> Option<Self> {
        let candidates = [
            Self::SharedBark,
            Self::ArmsDescription,
            Self::ArmsSell,
            Self::Sage, // before Tavern so the overlap resolves to Sage
            Self::Tavern,
            Self::HorseTrader,
            Self::ShipBroker,
            Self::Reagent,
            Self::Guild,
            Self::Healer,
            Self::Innkeeper,
        ];
        candidates.into_iter().find(|band| band.contains(id))
    }

    /// Select one record-id from the band by hashing the supplied
    /// seed against the band's slot count. Slots are walked across
    /// the band's sub-runs in order, so a holed band never returns a
    /// record it does not own.
    pub const fn pick_by_seed(self, seed: u64) -> usize {
        let bands = self.bands();
        let mut slot = (seed % (self.len() as u64)) as usize;
        let mut index = 0;
        while index < bands.len() {
            let (first, last) = bands[index];
            let width = last - first + 1;
            if slot < width {
                return first + slot;
            }
            slot -= width;
            index += 1;
        }
        // `len()` bounds `slot` to the summed width, so the loop
        // always returns; fall back to the last owned record rather
        // than panicking inside a const fn.
        bands[bands.len() - 1].1
    }
}

/// Pick a SHOPPE.DAT record id for the shop kind identified by the
/// Talk-resolved trigger byte (`shops.md §2`). Returns the first
/// record id in the matching band, suitable as a default greeting /
/// bark. Returns `None` for non-shop dialog ids.
pub const fn shoppe_default_record_for_dialog_id(dialog_id: u8) -> Option<usize> {
    let band = match dialog_id {
        0x81 => ShoppeBand::ArmsDescription,
        0x82 => ShoppeBand::Tavern,
        0x83 => ShoppeBand::HorseTrader,
        0x84 => ShoppeBand::ShipBroker,
        0x85 => ShoppeBand::Reagent,
        0x86 => ShoppeBand::Guild,
        0x87 => ShoppeBand::Healer,
        0x88 => ShoppeBand::Innkeeper,
        _ => return None,
    };
    Some(band.first())
}

/// Pick a shared-bark record id by hashing the supplied seed.
pub const fn shared_bark_record_by_seed(seed: u64) -> usize {
    ShoppeBand::SharedBark.pick_by_seed(seed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedShopBarkKind {
    Preamble,
    InitialGreeting,
    Farewell,
}

const SHARED_SHOP_BARK_UNUSED: usize = usize::MAX;

const SHARED_SHOP_PREAMBLE_FIRST: [usize; 8] =
    [SHARED_SHOP_BARK_UNUSED, 57, 92, 105, 127, 148, 165, 174];
const SHARED_SHOP_INITIAL_FIRST: [usize; 8] = [0, 61, 96, 109, 131, 152, 169, 178];
const SHARED_SHOP_FAREWELL_FIRST: [usize; 8] = [4, 65, 100, 113, 135, 156, 169, 182];

pub const fn shop_trigger_row(dialog_id: u8) -> Option<usize> {
    if dialog_id >= 0x81 && dialog_id <= 0x88 {
        Some((dialog_id - 0x81) as usize)
    } else {
        None
    }
}

pub const fn shared_shop_bark_record(
    dialog_id: u8,
    kind: SharedShopBarkKind,
    ordinal: u8,
) -> Option<usize> {
    let Some(row) = shop_trigger_row(dialog_id) else {
        return None;
    };
    let first = match kind {
        SharedShopBarkKind::Preamble => SHARED_SHOP_PREAMBLE_FIRST[row],
        SharedShopBarkKind::InitialGreeting => SHARED_SHOP_INITIAL_FIRST[row],
        SharedShopBarkKind::Farewell => SHARED_SHOP_FAREWELL_FIRST[row],
    };
    if first == SHARED_SHOP_BARK_UNUSED || ordinal > 3 {
        None
    } else {
        Some(first + ordinal as usize)
    }
}

/// `shops.md §8.A`: which Talk-entry shop flows render one of the four
/// shared entry-greeting records, chosen by a uniform `0..3` draw taken at
/// the moment the greeting is rendered.
///
/// The tavern trigger `0x82` is on this list. `§8.A`'s "Tavern drink flow"
/// row reads "Shared tavern arrival records `57..60` ... Arrival draws
/// uniformly from `57..60`", and its transcript row adds that the greeting
/// "Appends in the inherited conversation text window; there is no entry
/// clear". The earlier contract - tavern arrival deterministic by menu
/// state, with a text-window clear before it - is withdrawn
/// (`RETRACTIONS.md` R294); only the list records `69..72` and the
/// follow-up records `73..76` are deterministic from the state.
///
/// `0x81` arms "does not use this shared entry greeting" and `0x84`/`0x88`
/// print their own branch text, so all three stay off the list.
pub const fn talk_entry_uses_shared_preamble(dialog_id: u8) -> bool {
    matches!(dialog_id, 0x82 | 0x83 | 0x85 | 0x86 | 0x87)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_ranges_match_published_constants() {
        assert_eq!(ShoppeBand::SharedBark.bands(), &[(0, 7)]);
        assert_eq!(ShoppeBand::SharedBark.first(), 0);
        assert_eq!(ShoppeBand::SharedBark.last(), 7);
        assert!(ShoppeBand::SharedBark.contains(0));
        assert!(ShoppeBand::SharedBark.contains(7));
        assert!(!ShoppeBand::SharedBark.contains(8));
    }

    #[test]
    fn published_non_contiguous_bands_exclude_their_holes() {
        // `formats/shoppe-dat.md §6`: sage owns 84-88 and 91; the
        // records 89 and 90 between them are tavern branches.
        assert_eq!(ShoppeBand::Sage.bands(), &[(84, 88), (91, 91)]);
        assert!(ShoppeBand::Sage.contains(84));
        assert!(ShoppeBand::Sage.contains(88));
        assert!(!ShoppeBand::Sage.contains(89));
        assert!(!ShoppeBand::Sage.contains(90));
        assert!(ShoppeBand::Sage.contains(91));
        assert_eq!(ShoppeBand::Sage.len(), 6);

        // The tavern band runs 57-91 and owns the two sage holes.
        assert_eq!(ShoppeBand::Tavern.bands(), &[(57, 91)]);
        assert!(ShoppeBand::Tavern.contains(89));
        assert!(ShoppeBand::Tavern.contains(90));
        assert!(ShoppeBand::Tavern.contains(91));

        // `formats/shoppe-dat.md §6`: healer owns 163 and 165-173.
        assert_eq!(ShoppeBand::Healer.bands(), &[(163, 163), (165, 173)]);
        assert!(ShoppeBand::Healer.contains(163));
        assert!(!ShoppeBand::Healer.contains(SHOPPE_RECORDS_HEALER_EXCLUDED));
        assert!(ShoppeBand::Healer.contains(165));
        assert!(ShoppeBand::Healer.contains(173));
        assert_eq!(ShoppeBand::Healer.len(), 10);
    }

    #[test]
    fn pick_by_seed_never_returns_a_hole_in_a_holed_band() {
        for band in [ShoppeBand::Sage, ShoppeBand::Healer] {
            for seed in 0u64..64 {
                let id = band.pick_by_seed(seed);
                assert!(band.contains(id), "{band:?} seed {seed} -> id {id}");
            }
        }
        // Every owned slot is reachable.
        let reached: std::collections::BTreeSet<usize> = (0u64..64)
            .map(|seed| ShoppeBand::Sage.pick_by_seed(seed))
            .collect();
        assert_eq!(reached.len(), ShoppeBand::Sage.len());
    }

    #[test]
    fn band_lengths_are_inclusive_widths() {
        assert_eq!(ShoppeBand::SharedBark.len(), 8);
        assert_eq!(
            ShoppeBand::ArmsDescription.len(),
            SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST - SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST + 1
        );
    }

    #[test]
    fn classify_returns_the_matching_band_for_each_id() {
        assert_eq!(ShoppeBand::classify(0), Some(ShoppeBand::SharedBark));
        assert_eq!(
            ShoppeBand::classify(ShoppeBand::Healer.first()),
            Some(ShoppeBand::Healer)
        );
        assert_eq!(
            ShoppeBand::classify(SHOPPE_RECORDS_INNKEEPER_LAST),
            Some(ShoppeBand::Innkeeper)
        );
    }

    #[test]
    fn classify_resolves_sage_tavern_overlap_to_sage() {
        // `formats/shoppe-dat.md §6`: the sage records are
        // interleaved inside the tavern band (57-91). The classifier
        // prefers the narrower Sage band where both own the record,
        // and falls through to Tavern for 89/90, which the sage does
        // not own.
        assert_eq!(ShoppeBand::classify(84), Some(ShoppeBand::Sage));
        assert_eq!(ShoppeBand::classify(88), Some(ShoppeBand::Sage));
        assert_eq!(ShoppeBand::classify(89), Some(ShoppeBand::Tavern));
        assert_eq!(ShoppeBand::classify(90), Some(ShoppeBand::Tavern));
        assert_eq!(ShoppeBand::classify(91), Some(ShoppeBand::Sage));
    }

    #[test]
    fn pick_by_seed_stays_within_band() {
        for seed in 0u64..32 {
            let id = ShoppeBand::Tavern.pick_by_seed(seed);
            assert!(ShoppeBand::Tavern.contains(id), "seed {seed} → id {id}");
        }
    }

    #[test]
    fn dialog_id_dispatch_maps_to_correct_first_record() {
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x81),
            Some(SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST)
        );
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x82),
            Some(SHOPPE_RECORDS_TAVERN_FIRST)
        );
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x88),
            Some(SHOPPE_RECORDS_INNKEEPER_FIRST)
        );
    }

    #[test]
    fn dialog_id_dispatch_rejects_non_shop_ids() {
        assert_eq!(shoppe_default_record_for_dialog_id(0x00), None);
        assert_eq!(shoppe_default_record_for_dialog_id(0x80), None);
        assert_eq!(shoppe_default_record_for_dialog_id(0x89), None);
    }

    #[test]
    fn shared_bark_record_by_seed_returns_within_shared_band() {
        for seed in 0u64..32 {
            let id = shared_bark_record_by_seed(seed);
            assert!(ShoppeBand::SharedBark.contains(id));
        }
    }

    #[test]
    fn shared_shop_bark_rows_match_published_table() {
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::Preamble, 0),
            None
        );
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::InitialGreeting, 0),
            Some(0)
        );
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::Farewell, 3),
            Some(7)
        );
        assert_eq!(
            shared_shop_bark_record(0x82, SharedShopBarkKind::Preamble, 3),
            Some(60)
        );
        assert_eq!(
            shared_shop_bark_record(0x83, SharedShopBarkKind::InitialGreeting, 2),
            Some(98)
        );
        assert_eq!(
            shared_shop_bark_record(0x84, SharedShopBarkKind::Farewell, 1),
            Some(114)
        );
        assert_eq!(
            shared_shop_bark_record(0x85, SharedShopBarkKind::Preamble, 0),
            Some(127)
        );
        assert_eq!(
            shared_shop_bark_record(0x86, SharedShopBarkKind::InitialGreeting, 3),
            Some(155)
        );
        assert_eq!(
            shared_shop_bark_record(0x87, SharedShopBarkKind::Farewell, 0),
            Some(169)
        );
        assert_eq!(
            shared_shop_bark_record(0x88, SharedShopBarkKind::Farewell, 3),
            Some(185)
        );
        assert_eq!(
            shared_shop_bark_record(0x88, SharedShopBarkKind::Farewell, 4),
            None
        );
    }

    #[test]
    fn talk_entry_preamble_is_limited_to_published_shop_flows() {
        assert!(!talk_entry_uses_shared_preamble(0x81));
        // `shops.md §8.A` (`RETRACTIONS.md` R294): tavern arrival is a
        // uniform draw over the shared entry-greeting row 57..60.
        assert!(talk_entry_uses_shared_preamble(0x82));
        assert_eq!(
            shared_shop_bark_record(0x82, SharedShopBarkKind::Preamble, 0),
            Some(57)
        );
        assert_eq!(
            shared_shop_bark_record(0x82, SharedShopBarkKind::Preamble, 3),
            Some(60)
        );
        assert!(talk_entry_uses_shared_preamble(0x83));
        assert!(!talk_entry_uses_shared_preamble(0x84));
        assert!(talk_entry_uses_shared_preamble(0x85));
        assert!(talk_entry_uses_shared_preamble(0x86));
        assert!(talk_entry_uses_shared_preamble(0x87));
        assert!(!talk_entry_uses_shared_preamble(0x88));
    }

    #[test]
    fn every_shop_band_is_classifiable_by_its_first_id() {
        for band in [
            ShoppeBand::ArmsDescription,
            ShoppeBand::ArmsSell,
            ShoppeBand::Tavern,
            ShoppeBand::HorseTrader,
            ShoppeBand::ShipBroker,
            ShoppeBand::Reagent,
            ShoppeBand::Guild,
            ShoppeBand::Healer,
            ShoppeBand::Innkeeper,
        ] {
            let id = band.first();
            assert!(band.contains(id));
        }
    }
}
