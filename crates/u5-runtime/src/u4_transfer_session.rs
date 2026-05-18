//! Ultima IV → Ultima V transfer session state machine per
//! `systems/u4-transfer.md`.
//!
//! Wraps the per-attribute translation helpers in
//! [`crate::u4_transfer`] into a multi-step interactive flow:
//! file detection → validation → preview → commit/abort.

use crate::u4_transfer::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum U4TransferPhase {
    #[default]
    Idle,
    AwaitingSourceFile,
    Validating,
    PresentingPreview,
    AwaitingConfirmation,
    Committing,
    Done,
    Aborted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum U4TransferEvent {
    SourceFileMissing,
    SourceFileInvalid(&'static str),
    NoTransferableData,
    PreviewReady {
        name: String,
        class_index: u8,
        strength: u8,
        dexterity: u8,
        intelligence: u8,
    },
    Committed {
        gold: u16,
    },
    Aborted,
    AwaitingInput,
}

#[derive(Clone, Copy, Debug)]
pub enum U4TransferInput {
    SourceFileLoaded,
    ValidationOk,
    ValidationFailed(&'static str),
    NoTransferableData,
    Confirm(bool),
}

#[derive(Clone, Debug)]
pub struct U4TransferSession {
    pub phase: U4TransferPhase,
    pub preview: Option<U4TransferPreview>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4TransferPreview {
    pub name: String,
    pub class_index: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub gold: u16,
}

impl U4TransferSession {
    pub fn new() -> Self {
        Self {
            phase: U4TransferPhase::AwaitingSourceFile,
            preview: None,
        }
    }

    pub fn step(&mut self, input: U4TransferInput) -> U4TransferEvent {
        match (self.phase, input) {
            (U4TransferPhase::AwaitingSourceFile, U4TransferInput::SourceFileLoaded) => {
                self.phase = U4TransferPhase::Validating;
                U4TransferEvent::AwaitingInput
            }
            (U4TransferPhase::Validating, U4TransferInput::ValidationOk) => {
                self.phase = U4TransferPhase::PresentingPreview;
                U4TransferEvent::AwaitingInput
            }
            (U4TransferPhase::Validating, U4TransferInput::ValidationFailed(reason)) => {
                self.phase = U4TransferPhase::Aborted;
                U4TransferEvent::SourceFileInvalid(reason)
            }
            (U4TransferPhase::Validating, U4TransferInput::NoTransferableData) => {
                self.phase = U4TransferPhase::Aborted;
                U4TransferEvent::NoTransferableData
            }
            (U4TransferPhase::PresentingPreview, _) => {
                // Caller has populated preview via `set_preview`; emit it.
                if let Some(p) = self.preview.clone() {
                    self.phase = U4TransferPhase::AwaitingConfirmation;
                    U4TransferEvent::PreviewReady {
                        name: p.name,
                        class_index: p.class_index,
                        strength: p.strength,
                        dexterity: p.dexterity,
                        intelligence: p.intelligence,
                    }
                } else {
                    U4TransferEvent::AwaitingInput
                }
            }
            (U4TransferPhase::AwaitingConfirmation, U4TransferInput::Confirm(true)) => {
                self.phase = U4TransferPhase::Done;
                let gold = self.preview.as_ref().map(|p| p.gold).unwrap_or(0);
                U4TransferEvent::Committed { gold }
            }
            (U4TransferPhase::AwaitingConfirmation, U4TransferInput::Confirm(false)) => {
                self.phase = U4TransferPhase::Aborted;
                U4TransferEvent::Aborted
            }
            _ => U4TransferEvent::AwaitingInput,
        }
    }

    pub fn set_preview(&mut self, preview: U4TransferPreview) {
        self.preview = Some(preview);
    }

    pub fn is_done(&self) -> bool {
        matches!(self.phase, U4TransferPhase::Done | U4TransferPhase::Aborted)
    }
}

/// Build a [`U4TransferPreview`] from raw U4 attribute values, using
/// the published stat-translation table.
pub fn u4_transfer_preview_from_u4_values(
    name: String,
    class_index: u8,
    u4_strength: u16,
    u4_dexterity: u16,
    u4_intelligence: u16,
    gold: u16,
) -> U4TransferPreview {
    let strength = u4_transfer_attribute_to_u5(u4_strength).max(U4_TRANSFER_STRENGTH_FLOOR);
    let dexterity = u4_transfer_attribute_to_u5(u4_dexterity);
    let intelligence = u4_transfer_attribute_to_u5(u4_intelligence);
    U4TransferPreview {
        name,
        class_index,
        strength,
        dexterity,
        intelligence,
        gold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_awaiting_source_file() {
        let s = U4TransferSession::new();
        assert_eq!(s.phase, U4TransferPhase::AwaitingSourceFile);
    }

    #[test]
    fn source_file_loaded_transitions_to_validating() {
        let mut s = U4TransferSession::new();
        s.step(U4TransferInput::SourceFileLoaded);
        assert_eq!(s.phase, U4TransferPhase::Validating);
    }

    #[test]
    fn validation_failure_aborts_session() {
        let mut s = U4TransferSession::new();
        s.step(U4TransferInput::SourceFileLoaded);
        let event = s.step(U4TransferInput::ValidationFailed("bad bytes"));
        assert_eq!(event, U4TransferEvent::SourceFileInvalid("bad bytes"));
        assert!(s.is_done());
    }

    #[test]
    fn no_transferable_data_aborts_session() {
        let mut s = U4TransferSession::new();
        s.step(U4TransferInput::SourceFileLoaded);
        let event = s.step(U4TransferInput::NoTransferableData);
        assert_eq!(event, U4TransferEvent::NoTransferableData);
        assert!(s.is_done());
    }

    #[test]
    fn preview_then_confirm_commits_transfer_with_gold() {
        let mut s = U4TransferSession::new();
        s.set_preview(U4TransferPreview {
            name: "Avatar".to_string(),
            class_index: 0,
            strength: 25,
            dexterity: 20,
            intelligence: 18,
            gold: 200,
        });
        s.step(U4TransferInput::SourceFileLoaded);
        s.step(U4TransferInput::ValidationOk);
        let preview_event = s.step(U4TransferInput::Confirm(true));
        assert!(matches!(preview_event, U4TransferEvent::PreviewReady { .. }));
        let event = s.step(U4TransferInput::Confirm(true));
        assert!(matches!(event, U4TransferEvent::Committed { gold: 200 }));
    }

    #[test]
    fn preview_then_decline_aborts_transfer() {
        let mut s = U4TransferSession::new();
        s.set_preview(U4TransferPreview {
            name: "Cal".to_string(),
            class_index: 2,
            strength: 25,
            dexterity: 12,
            intelligence: 12,
            gold: 0,
        });
        s.step(U4TransferInput::SourceFileLoaded);
        s.step(U4TransferInput::ValidationOk);
        s.step(U4TransferInput::Confirm(true));
        let event = s.step(U4TransferInput::Confirm(false));
        assert_eq!(event, U4TransferEvent::Aborted);
    }

    #[test]
    fn u4_to_u5_stat_translation_applies_strength_floor() {
        let preview =
            u4_transfer_preview_from_u4_values("X".to_string(), 0, 5, 5, 5, 0);
        assert_eq!(preview.strength, U4_TRANSFER_STRENGTH_FLOOR);
    }
}
