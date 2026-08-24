//! Required-disk and disk-error prompt session state.
//!
//! `systems/disk-prompt.md` withdraws the earlier interpretation of
//! these bytes as a screen/presentation-mode controller. They are the
//! distribution-disk selector, its per-disk drive table, and the
//! critical-error handler state. Modern single-directory installs can
//! make drive selection a no-op, but keeping this model preserves the
//! retry ordering and the original session-only state transitions.

use std::error::Error;
use std::fmt;

pub const REQUIRED_DISK_TABLE_LEN: usize = 6;
pub const REQUIRED_DISK_PROGRAM_INDEX: u8 = 0;
pub const REQUIRED_DISK_BRITANNIA_INDEX: u8 = 1;
pub const REQUIRED_DISK_ALIAS_A_INDEX: u8 = 2;
pub const REQUIRED_DISK_U5_SAVE_INDEX: u8 = 3;
pub const REQUIRED_DISK_U4_PLAYER_INDEX: u8 = 4;
pub const REQUIRED_DISK_ALIAS_B_INDEX: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredDisk {
    Program,
    Britannia,
    UltimaVSave,
    UltimaIvPlayer,
}

impl RequiredDisk {
    pub const fn index(self) -> u8 {
        match self {
            Self::Program => REQUIRED_DISK_PROGRAM_INDEX,
            Self::Britannia => REQUIRED_DISK_BRITANNIA_INDEX,
            Self::UltimaVSave => REQUIRED_DISK_U5_SAVE_INDEX,
            Self::UltimaIvPlayer => REQUIRED_DISK_U4_PLAYER_INDEX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedRequiredDiskIndex(pub u8);

impl fmt::Display for UnsupportedRequiredDiskIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported required-disk index {}", self.0)
    }
}

impl Error for UnsupportedRequiredDiskIndex {}

/// `disk-prompt.md §2`/§7: request indices 2 and 5 are historical
/// aliases for Britannia disk index 1. Only the four canonical roles
/// can become the resident required-disk value.
pub const fn required_disk_from_request_index(
    requested_index: u8,
) -> Result<RequiredDisk, UnsupportedRequiredDiskIndex> {
    match requested_index {
        REQUIRED_DISK_PROGRAM_INDEX => Ok(RequiredDisk::Program),
        REQUIRED_DISK_BRITANNIA_INDEX
        | REQUIRED_DISK_ALIAS_A_INDEX
        | REQUIRED_DISK_ALIAS_B_INDEX => Ok(RequiredDisk::Britannia),
        REQUIRED_DISK_U5_SAVE_INDEX => Ok(RequiredDisk::UltimaVSave),
        REQUIRED_DISK_U4_PLAYER_INDEX => Ok(RequiredDisk::UltimaIvPlayer),
        other => Err(UnsupportedRequiredDiskIndex(other)),
    }
}

pub const fn canonical_required_disk_index(
    requested_index: u8,
) -> Result<u8, UnsupportedRequiredDiskIndex> {
    match required_disk_from_request_index(requested_index) {
        Ok(disk) => Ok(disk.index()),
        Err(err) => Err(err),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DosDrive(u8);

impl DosDrive {
    pub const FIRST_FLOPPY: Self = Self(b'A');
    pub const SECOND_FLOPPY: Self = Self(b'B');
    pub const SINGLE_DIRECTORY: Self = Self(b'C');

    pub const fn from_ascii(letter: u8) -> Option<Self> {
        let upper = if letter >= b'a' && letter <= b'z' {
            letter - (b'a' - b'A')
        } else {
            letter
        };
        if upper >= b'A' && upper <= b'Z' {
            Some(Self(upper))
        } else {
            None
        }
    }

    pub const fn ascii(self) -> u8 {
        self.0
    }

    pub const fn is_floppy(self) -> bool {
        self.0 == b'A' || self.0 == b'B'
    }

    pub const fn is_fixed(self) -> bool {
        !self.is_floppy()
    }

    pub const fn other_floppy(self) -> Option<Self> {
        match self.0 {
            b'A' => Some(Self::SECOND_FLOPPY),
            b'B' => Some(Self::FIRST_FLOPPY),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiskPromptPresentation {
    #[default]
    PlainConsole,
    PictureOverlay,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiskErrorHandler {
    #[default]
    InsertDiskPrompt,
    ImmediateReturnGuard,
    WriteProtectPrompt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskRequestOutcome {
    DriveUnknown,
    SelectKnownDrive(DosDrive),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskErrorAction {
    Suppressed,
    SilentRetry {
        drive: DosDrive,
    },
    FixedDiskFallback {
        drive: DosDrive,
    },
    Prompt {
        disk: RequiredDisk,
        known_drive: Option<DosDrive>,
        presentation: DiskPromptPresentation,
    },
    WriteProtectPrompt {
        presentation: DiskPromptPresentation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptSelectionOutcome {
    RePrompt,
    Accepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskOperationFamily {
    ProgramResources,
    GameplayResources,
    UltimaVSaveFiles,
    UltimaIvTransferFiles,
}

impl DiskOperationFamily {
    pub const fn required_disk(self) -> RequiredDisk {
        match self {
            Self::ProgramResources => RequiredDisk::Program,
            Self::GameplayResources => RequiredDisk::Britannia,
            Self::UltimaVSaveFiles => RequiredDisk::UltimaVSave,
            Self::UltimaIvTransferFiles => RequiredDisk::UltimaIvPlayer,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskPromptSession {
    required_disk: RequiredDisk,
    drive_table: [Option<DosDrive>; REQUIRED_DISK_TABLE_LEN],
    already_prompted: Option<RequiredDisk>,
    presentation: DiskPromptPresentation,
    floppy_count: u8,
    handler: DiskErrorHandler,
    guarded_handler: Option<DiskErrorHandler>,
}

impl DiskPromptSession {
    /// `disk-prompt.md §6`: boot starts on the Program disk with one
    /// floppy drive. A fixed current drive is copied to every table
    /// entry; a floppy current drive leaves every other disk unknown.
    pub fn boot(current_drive: DosDrive) -> Self {
        let mut drive_table = [None; REQUIRED_DISK_TABLE_LEN];
        drive_table[usize::from(REQUIRED_DISK_PROGRAM_INDEX)] = Some(current_drive);
        if current_drive.is_fixed() {
            drive_table.fill(Some(current_drive));
        }
        Self {
            required_disk: RequiredDisk::Program,
            drive_table,
            already_prompted: None,
            presentation: DiskPromptPresentation::PlainConsole,
            floppy_count: 1,
            handler: DiskErrorHandler::InsertDiskPrompt,
            guarded_handler: None,
        }
    }

    /// Compatibility state for the engine's normal mounted-directory
    /// runtime. All disk roles resolve to one virtual fixed drive, so
    /// callers preserve request/retry state without displaying a swap.
    pub fn single_directory() -> Self {
        Self::boot(DosDrive::SINGLE_DIRECTORY)
    }

    pub const fn required_disk(&self) -> RequiredDisk {
        self.required_disk
    }

    pub const fn already_prompted(&self) -> Option<RequiredDisk> {
        self.already_prompted
    }

    pub const fn presentation(&self) -> DiskPromptPresentation {
        self.presentation
    }

    pub fn set_presentation(&mut self, presentation: DiskPromptPresentation) {
        self.presentation = presentation;
    }

    pub const fn floppy_count(&self) -> u8 {
        self.floppy_count
    }

    pub const fn handler(&self) -> DiskErrorHandler {
        self.handler
    }

    pub fn drive_for(&self, disk: RequiredDisk) -> Option<DosDrive> {
        self.drive_table[usize::from(disk.index())]
    }

    /// `disk-prompt.md §7`: a request stores the canonical disk role.
    /// A known drive is selected immediately and invalidates the
    /// already-prompted cache; an unknown drive does not prompt here.
    pub fn request_disk(
        &mut self,
        requested_index: u8,
    ) -> Result<DiskRequestOutcome, UnsupportedRequiredDiskIndex> {
        let disk = required_disk_from_request_index(requested_index)?;
        self.required_disk = disk;
        match self.drive_for(disk) {
            Some(drive) => {
                self.already_prompted = None;
                Ok(DiskRequestOutcome::SelectKnownDrive(drive))
            }
            None => Ok(DiskRequestOutcome::DriveUnknown),
        }
    }

    pub fn request_operation(&mut self, operation: DiskOperationFamily) -> DiskRequestOutcome {
        self.request_disk(operation.required_disk().index())
            .expect("operation families always map to canonical disk indices")
    }

    pub fn install_write_error_handler(&mut self) {
        debug_assert!(self.guarded_handler.is_none());
        self.handler = DiskErrorHandler::WriteProtectPrompt;
    }

    pub fn restore_insert_disk_handler(&mut self) {
        debug_assert!(self.guarded_handler.is_none());
        self.handler = DiskErrorHandler::InsertDiskPrompt;
    }

    /// Installs the recursion guard. A second error while the guard is
    /// live is swallowed, exactly as §4 requires.
    pub fn begin_error_handling(&mut self) -> bool {
        if self.handler == DiskErrorHandler::ImmediateReturnGuard {
            return false;
        }
        debug_assert!(self.guarded_handler.is_none());
        self.guarded_handler = Some(self.handler);
        self.handler = DiskErrorHandler::ImmediateReturnGuard;
        true
    }

    pub fn current_error_action(&mut self) -> DiskErrorAction {
        let Some(guarded_handler) = self.guarded_handler else {
            return DiskErrorAction::Suppressed;
        };
        match guarded_handler {
            DiskErrorHandler::ImmediateReturnGuard => DiskErrorAction::Suppressed,
            DiskErrorHandler::WriteProtectPrompt => DiskErrorAction::WriteProtectPrompt {
                presentation: self.presentation,
            },
            DiskErrorHandler::InsertDiskPrompt => self.insert_disk_action(),
        }
    }

    pub fn end_error_handling(&mut self) {
        if let Some(handler) = self.guarded_handler.take() {
            self.handler = handler;
        }
    }

    fn insert_disk_action(&mut self) -> DiskErrorAction {
        let disk = self.required_disk;
        let recorded_drive = self.drive_for(disk);
        if let Some(drive) = recorded_drive {
            if drive.is_floppy() && self.floppy_count > 1 {
                let other = drive
                    .other_floppy()
                    .expect("the floppy predicate guarantees an alternate drive");
                self.drive_table[usize::from(disk.index())] = Some(other);
                if self.already_prompted != Some(disk) {
                    self.already_prompted = Some(disk);
                    return DiskErrorAction::SilentRetry { drive: other };
                }
            }
            if drive.is_fixed() {
                self.drive_table[usize::from(disk.index())] = Some(DosDrive::FIRST_FLOPPY);
                self.already_prompted = None;
                return DiskErrorAction::FixedDiskFallback {
                    drive: DosDrive::FIRST_FLOPPY,
                };
            }
        }
        DiskErrorAction::Prompt {
            disk,
            known_drive: self.drive_for(disk),
            presentation: self.presentation,
        }
    }

    /// Handle the visible prompt's key. A known-drive prompt treats any
    /// key as acknowledgement and leaves its newly selected drive alone.
    /// An unknown-drive prompt rejects nonletters before DOS selection;
    /// an accepted letter updates the table and applies the index-3 to
    /// index-1 propagation.
    pub fn record_prompt_key(
        &mut self,
        key: u8,
        drive_selection_accepted: bool,
    ) -> PromptSelectionOutcome {
        let disk = self.required_disk;
        if self.drive_for(disk).is_some() {
            self.already_prompted = Some(disk);
            return PromptSelectionOutcome::Accepted;
        }
        let Some(drive) = DosDrive::from_ascii(key) else {
            return PromptSelectionOutcome::RePrompt;
        };
        if !drive_selection_accepted {
            return PromptSelectionOutcome::RePrompt;
        }
        self.drive_table[usize::from(disk.index())] = Some(drive);
        if drive == DosDrive::SECOND_FLOPPY {
            self.floppy_count = 2;
        }
        if disk == RequiredDisk::UltimaVSave {
            self.drive_table[usize::from(REQUIRED_DISK_BRITANNIA_INDEX)] = Some(drive);
        }
        self.already_prompted = Some(disk);
        PromptSelectionOutcome::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_indices_two_and_five_fold_to_britannia_one() {
        assert_eq!(
            required_disk_from_request_index(REQUIRED_DISK_ALIAS_A_INDEX),
            Ok(RequiredDisk::Britannia)
        );
        assert_eq!(
            required_disk_from_request_index(REQUIRED_DISK_ALIAS_B_INDEX),
            Ok(RequiredDisk::Britannia)
        );
        assert_eq!(canonical_required_disk_index(2), Ok(1));
        assert_eq!(canonical_required_disk_index(5), Ok(1));
        assert_eq!(
            required_disk_from_request_index(6),
            Err(UnsupportedRequiredDiskIndex(6))
        );
    }

    #[test]
    fn fixed_disk_boot_seeds_every_role_while_floppy_boot_leaves_others_unknown() {
        let fixed = DiskPromptSession::boot(DosDrive::SINGLE_DIRECTORY);
        for disk in [
            RequiredDisk::Program,
            RequiredDisk::Britannia,
            RequiredDisk::UltimaVSave,
            RequiredDisk::UltimaIvPlayer,
        ] {
            assert_eq!(fixed.drive_for(disk), Some(DosDrive::SINGLE_DIRECTORY));
        }

        let floppy = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
        assert_eq!(
            floppy.drive_for(RequiredDisk::Program),
            Some(DosDrive::FIRST_FLOPPY)
        );
        assert_eq!(floppy.drive_for(RequiredDisk::Britannia), None);
        assert_eq!(floppy.floppy_count(), 1);
    }

    #[test]
    fn disk_request_selects_known_drive_but_never_prompts_for_unknown_drive() {
        let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
        assert_eq!(
            session.request_disk(REQUIRED_DISK_PROGRAM_INDEX),
            Ok(DiskRequestOutcome::SelectKnownDrive(DosDrive::FIRST_FLOPPY))
        );
        assert_eq!(
            session.request_disk(REQUIRED_DISK_BRITANNIA_INDEX),
            Ok(DiskRequestOutcome::DriveUnknown)
        );
        assert_eq!(session.required_disk(), RequiredDisk::Britannia);
    }

    #[test]
    fn fixed_disk_error_falls_back_to_first_floppy_and_invalidates_cache() {
        let mut session = DiskPromptSession::single_directory();
        session.request_disk(REQUIRED_DISK_BRITANNIA_INDEX).unwrap();
        assert!(session.begin_error_handling());
        assert_eq!(
            session.current_error_action(),
            DiskErrorAction::FixedDiskFallback {
                drive: DosDrive::FIRST_FLOPPY
            }
        );
        assert_eq!(
            session.drive_for(RequiredDisk::Britannia),
            Some(DosDrive::FIRST_FLOPPY)
        );
        assert_eq!(session.already_prompted(), None);
        session.end_error_handling();
        assert_eq!(session.handler(), DiskErrorHandler::InsertDiskPrompt);
    }

    #[test]
    fn accepted_second_floppy_for_index_three_propagates_to_britannia_one() {
        let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
        session.request_disk(REQUIRED_DISK_U5_SAVE_INDEX).unwrap();
        assert!(session.begin_error_handling());
        assert_eq!(
            session.current_error_action(),
            DiskErrorAction::Prompt {
                disk: RequiredDisk::UltimaVSave,
                known_drive: None,
                presentation: DiskPromptPresentation::PlainConsole,
            }
        );
        assert_eq!(
            session.record_prompt_key(b'B', true),
            PromptSelectionOutcome::Accepted
        );
        assert_eq!(session.floppy_count(), 2);
        assert_eq!(
            session.drive_for(RequiredDisk::UltimaVSave),
            Some(DosDrive::SECOND_FLOPPY)
        );
        assert_eq!(
            session.drive_for(RequiredDisk::Britannia),
            Some(DosDrive::SECOND_FLOPPY)
        );
    }

    #[test]
    fn recursion_guard_swallows_nested_errors_and_restores_write_handler() {
        let mut session = DiskPromptSession::single_directory();
        session.install_write_error_handler();
        assert!(session.begin_error_handling());
        assert!(!session.begin_error_handling());
        assert_eq!(
            session.current_error_action(),
            DiskErrorAction::WriteProtectPrompt {
                presentation: DiskPromptPresentation::PlainConsole
            }
        );
        session.end_error_handling();
        assert_eq!(session.handler(), DiskErrorHandler::WriteProtectPrompt);
        session.restore_insert_disk_handler();
        assert_eq!(session.handler(), DiskErrorHandler::InsertDiskPrompt);
    }

    #[test]
    fn rejected_drive_selection_reprompts_without_mutating_the_table() {
        let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
        session.request_disk(REQUIRED_DISK_U4_PLAYER_INDEX).unwrap();
        assert_eq!(
            session.record_prompt_key(b'B', false),
            PromptSelectionOutcome::RePrompt
        );
        assert_eq!(session.drive_for(RequiredDisk::UltimaIvPlayer), None);
        assert_eq!(session.floppy_count(), 1);
    }

    #[test]
    fn same_cache_dual_floppy_failure_updates_drive_then_uses_known_prompt() {
        let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
        session.request_operation(DiskOperationFamily::UltimaVSaveFiles);
        assert!(session.begin_error_handling());
        assert!(matches!(
            session.current_error_action(),
            DiskErrorAction::Prompt {
                known_drive: None,
                ..
            }
        ));
        assert_eq!(
            session.record_prompt_key(b'B', true),
            PromptSelectionOutcome::Accepted
        );
        session.end_error_handling();

        assert!(session.begin_error_handling());
        assert_eq!(
            session.current_error_action(),
            DiskErrorAction::Prompt {
                disk: RequiredDisk::UltimaVSave,
                known_drive: Some(DosDrive::FIRST_FLOPPY),
                presentation: DiskPromptPresentation::PlainConsole,
            }
        );
        assert_eq!(
            session.record_prompt_key(b'?', false),
            PromptSelectionOutcome::Accepted
        );
        assert_eq!(
            session.drive_for(RequiredDisk::UltimaVSave),
            Some(DosDrive::FIRST_FLOPPY)
        );
    }

    #[test]
    fn operation_families_map_to_the_four_canonical_roles() {
        assert_eq!(
            DiskOperationFamily::ProgramResources.required_disk(),
            RequiredDisk::Program
        );
        assert_eq!(
            DiskOperationFamily::GameplayResources.required_disk(),
            RequiredDisk::Britannia
        );
        assert_eq!(
            DiskOperationFamily::UltimaVSaveFiles.required_disk(),
            RequiredDisk::UltimaVSave
        );
        assert_eq!(
            DiskOperationFamily::UltimaIvTransferFiles.required_disk(),
            RequiredDisk::UltimaIvPlayer
        );
    }
}
