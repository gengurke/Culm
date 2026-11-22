//! Maintenance: the cards' phases and jobs (F6).

use super::{State, err};
use crate::backend::Maintenance;
use crate::worker::{self, Msg};

/// A maintenance card's phase (F6).
#[derive(Debug, Clone, PartialEq)]
pub enum MaintPhase {
    Idle,
    Confirm,
    Running,
    Done(String),
    Failed(String),
}

/// Which maintenance card (the cache card's WDB toggle lives in `State::cache_wdb_only`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaintCard {
    Cache,
    Leftovers,
    Wtf,
}

impl MaintCard {
    /// Only cleaning up deletes something that can't be rebuilt: it asks first. A backup only adds a
    /// zip and the game rebuilds its cache, so those run at once.
    pub fn confirms(self) -> bool {
        self == MaintCard::Leftovers
    }

    pub fn job(self, wdb_only: bool) -> Maintenance {
        match self {
            MaintCard::Cache => Maintenance::ClearCache { wdb_only },
            MaintCard::Leftovers => Maintenance::CleanBackups,
            MaintCard::Wtf => Maintenance::BackupWtf,
        }
    }
}

impl State {
    pub(super) fn refresh_maintenance(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        worker::spawn(self, "maintenance-info", move |b, post| {
            post.send(Msg::GameRunning {
                running: b.game_running(&inst),
                inst: inst.id.clone(),
            });
            post.send(Msg::Backups {
                list: b.backups(&inst).unwrap_or_default(),
                inst: inst.id,
            });
        });
    }

    pub(super) fn maintain(&mut self, card: MaintCard) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        if self.game_running {
            self.maint.insert(
                card,
                MaintPhase::Failed("World of Warcraft is running".into()),
            );
            return;
        }
        self.maint.insert(card, MaintPhase::Running);
        let job = card.job(self.cache_wdb_only);
        worker::spawn(self, "maintenance", move |b, post| {
            let result = b.maintenance(&inst, job).map_err(err);
            post.send(Msg::Maintained {
                inst: inst.id.clone(),
                card,
                result,
            });
            if card == MaintCard::Wtf {
                post.send(Msg::Backups {
                    list: b.backups(&inst).unwrap_or_default(),
                    inst: inst.id,
                });
            }
        });
    }
}
