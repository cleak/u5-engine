//! Boot-time graphics-driver selection per `boot.md` §4-§5.

use crate::input_case_fold;

/// `launcher.md §2,§5` published file names that make up the
/// game-owned startup boundary. The DOS MZ image, the resident
/// shared data image, and the boot-time intro overlay are the
/// minimum executable-side files; display drivers and asset OVLs
/// are loaded later through their own paths.
pub const ULTIMA_EXE_FILENAME: &str = "ULTIMA.EXE";
pub const DATA_OVL_FILENAME: &str = "DATA.OVL";
pub const INTRO_OVL_FILENAME: &str = "INTRO.OVL";

/// `boot.md §4` graphics-capability classes the auto-detect path can
/// produce. The "EGA sentinel" variant marks an EGA extension present in
/// an unusual current mode; the loader treats it as unresolved and must
/// not be selected as a fifth driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsCapability {
    GenericFourColour,
    Ega,
    Tandy,
    Hercules,
    EgaSentinel,
}

/// `boot.md §3` machine-class probe outcome. Boot classifies the host
/// along the (machine, graphics) axes; this enum is the machine half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineClass {
    /// IBM PC or PCjr-style baseline. PCjr skips the extended graphics
    /// probe (so the auto-detect path must not run the Hercules port
    /// test on this class).
    PcOrPcjr,
    /// IBM AT or later compatible.
    At,
    /// Tandy 1000 detected by the conservative ROM signature scan that
    /// runs only for the BIOS id family that can hide a Tandy 1000
    /// behind a PC-compatible id.
    Tandy1000,
    /// Non-matching or clone BIOS id; treated as generic XT-class with
    /// graphics probing.
    OtherOrGenericXt,
}

impl MachineClass {
    /// `boot.md §3`: PCjr-class machines skip the extended-graphics
    /// probe; every other class runs the BIOS+Hercules probe.
    pub const fn skips_extended_graphics_probe(self) -> bool {
        matches!(self, Self::PcOrPcjr)
    }

    /// `boot.md §3`: a Tandy-1000 ROM-signature hit stamps both the
    /// machine class and the Tandy graphics capability up front,
    /// short-circuiting the ordinary auto-detect path.
    pub const fn forces_tandy_graphics(self) -> bool {
        matches!(self, Self::Tandy1000)
    }
}

/// `boot.md §5` driver families that can actually be loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayDriverFamily {
    /// `CGA.DRV`-style four-colour / monochrome fallback.
    Cga,
    /// `EGA.DRV` enhanced graphics.
    Ega,
    /// `T1K.DRV` Tandy 1000 16-colour path.
    Tandy,
    /// `HER.DRV` Hercules monochrome path.
    Hercules,
}

impl DisplayDriverFamily {
    /// Filename the boot loader reads for this driver family.
    pub const fn driver_filename(self) -> &'static str {
        match self {
            DisplayDriverFamily::Cga => "CGA.DRV",
            DisplayDriverFamily::Ega => "EGA.DRV",
            DisplayDriverFamily::Tandy => "T1K.DRV",
            DisplayDriverFamily::Hercules => "HER.DRV",
        }
    }

    /// `boot.md §5` / `intro.md`: paired screen-panel asset suffix
    /// the driver family chooses for the LZW screen-panel archives
    /// (`STARTSC`, `STORY1`–`STORY6`, etc.). EGA and Tandy are
    /// 16-colour and use `.16`; CGA and Hercules are 4-colour and
    /// use `.4`.
    pub const fn intro_panel_asset_suffix(self) -> &'static str {
        match self {
            DisplayDriverFamily::Ega | DisplayDriverFamily::Tandy => ".16",
            DisplayDriverFamily::Cga | DisplayDriverFamily::Hercules => ".4",
        }
    }

    /// `boot.md §5` / `intro.md`: returns `true` for the paired
    /// high-colour intro asset family (EGA and Tandy); `false` for
    /// the low-colour CGA and Hercules families.
    pub const fn uses_high_colour_intro_assets(self) -> bool {
        matches!(self, DisplayDriverFamily::Ega | DisplayDriverFamily::Tandy)
    }
}

/// `boot.md §5`: parse a command-line driver-selection argument. The
/// resident parser looks at the first character of the first argument,
/// case-folds it, and accepts only `C/E/T/H`. Anything else (including
/// no argument) leaves the explicit selector clear, which the caller
/// represents as `None` and falls back to auto-detection.
pub fn parse_explicit_driver_selector(arg: Option<&str>) -> Option<DisplayDriverFamily> {
    let arg = arg?;
    let first = arg.as_bytes().first().copied()?;
    match input_case_fold(first) {
        b'C' => Some(DisplayDriverFamily::Cga),
        b'E' => Some(DisplayDriverFamily::Ega),
        b'T' => Some(DisplayDriverFamily::Tandy),
        b'H' => Some(DisplayDriverFamily::Hercules),
        _ => None,
    }
}

/// `boot.md §5`: resolve the loaded driver family from the command-line
/// explicit selector and the auto-detected capability. Explicit wins;
/// otherwise the capability picks. Returns `None` for the EgaSentinel
/// case when no explicit selector rewrites it (the loader takes no
/// driver-load path).
pub fn resolve_driver_family(
    explicit: Option<DisplayDriverFamily>,
    detected: GraphicsCapability,
) -> Option<DisplayDriverFamily> {
    if let Some(family) = explicit {
        return Some(family);
    }
    match detected {
        GraphicsCapability::GenericFourColour => Some(DisplayDriverFamily::Cga),
        GraphicsCapability::Ega => Some(DisplayDriverFamily::Ega),
        GraphicsCapability::Tandy => Some(DisplayDriverFamily::Tandy),
        GraphicsCapability::Hercules => Some(DisplayDriverFamily::Hercules),
        GraphicsCapability::EgaSentinel => None,
    }
}

/// `boot.md §5`: Tandy-class systems with less than 368 KB of
/// conventional memory are downgraded to the generic low-colour path
/// before driver and asset selection are finalized.
pub const TANDY_LOW_MEMORY_THRESHOLD_KB: u16 = 368;
pub const fn tandy_low_memory_downgrades(conventional_memory_kb: u16) -> bool {
    conventional_memory_kb < TANDY_LOW_MEMORY_THRESHOLD_KB
}
