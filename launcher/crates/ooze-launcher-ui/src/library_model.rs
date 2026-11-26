//! Interface state for the library: a reducer over [`LibEvent`]s. Like `state::Model`, it holds no
//! business logic; it records what the backend reported and which operation the interface is
//! waiting for, so late results from abandoned operations are ignored.

use crate::library::*;
use ooze_ui::profile::Profile;

/// What a finished operation was, for wording its notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibOpKind {
    Instance,
    Profile,
    Theme,
    Search,
    Project,
    Worlds,
    Plan,
    Apply,
    InstalledList,
    Content,
    Integrity,
}

pub fn kind_of(command: &LibCommand) -> LibOpKind {
    use LibCommand as C;
    match command {
        C::Select(_)
        | C::Create { .. }
        | C::Rename { .. }
        | C::Duplicate { .. }
        | C::Install(_)
        | C::Archive(_) => LibOpKind::Instance,
        C::LoadProfile(_) | C::SaveProfile { .. } => LibOpKind::Profile,
        C::SaveTheme(_) => LibOpKind::Theme,
        C::Search(_) => LibOpKind::Search,
        C::Project { .. } => LibOpKind::Project,
        C::ListWorlds(_) => LibOpKind::Worlds,
        C::Plan { .. } => LibOpKind::Plan,
        C::Apply { .. } => LibOpKind::Apply,
        C::ListInstalled { .. } => LibOpKind::InstalledList,
        C::SetEnabled { .. } | C::Update { .. } | C::Remove { .. } => LibOpKind::Content,
        C::CheckIntegrity(_) | C::Resolve { .. } => LibOpKind::Integrity,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LibNotice {
    Error {
        during: LibOpKind,
        error: LibError,
    },
    Cancelled,
    /// Verified and recorded; never "working".
    Installed {
        kind: ContentKind,
        titles: Vec<String>,
    },
    ProfileSaved,
    ThemeSaved,
    ContentChanged,
    IntegrityResolved,
}

#[derive(Debug)]
pub struct LibraryModel {
    pub snap: LibrarySnapshot,
    pub open_error: Option<LibError>,
    active: Option<(LibOpId, LibCommand)>,
    pub progress: Option<LibProgress>,
    pub search: Option<SearchPage>,
    pub project: Option<ProjectDetails>,
    pub plan: Option<DependencyPlan>,
    pub worlds: Option<(InstanceId, Vec<World>)>,
    pub installed: Option<(InstanceId, ContentKind, Vec<InstalledItem>)>,
    pub profile: Option<(InstanceId, Profile, u64)>,
    pub integrity: Option<(InstanceId, IntegrityReport)>,
    pub notice: Option<LibNotice>,
    /// Increments with every new notice, for the polite live region.
    pub notice_seq: u64,
}

impl LibraryModel {
    pub fn new(snap: LibrarySnapshot, open_error: Option<LibError>) -> Self {
        Self {
            snap,
            open_error,
            active: None,
            progress: None,
            search: None,
            project: None,
            plan: None,
            worlds: None,
            installed: None,
            profile: None,
            integrity: None,
            notice: None,
            notice_seq: 0,
        }
    }

    pub fn caps(&self) -> &LibraryCapabilities {
        &self.snap.capabilities
    }

    pub fn busy(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<(LibOpId, LibOpKind)> {
        self.active.as_ref().map(|(id, c)| (*id, kind_of(c)))
    }

    pub fn active_command(&self) -> Option<&LibCommand> {
        self.active.as_ref().map(|(_, c)| c)
    }

    /// Cancellation is offered until the backend reports the commit boundary.
    pub fn cancellable(&self) -> bool {
        self.active.is_some()
            && self
                .progress
                .as_ref()
                .is_none_or(|p| p.step != LibStep::Committing)
    }

    fn set_notice(&mut self, notice: LibNotice) {
        self.notice = Some(notice);
        self.notice_seq += 1;
    }

    pub fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    pub fn on_submitted(&mut self, command: LibCommand, result: Result<LibOpId, LibError>) {
        match result {
            Ok(id) => {
                self.active = Some((id, command));
                self.progress = None;
            }
            Err(error) => {
                let during = kind_of(&command);
                self.set_notice(LibNotice::Error { during, error });
            }
        }
    }

    /// Stop waiting for the active operation (navigation or Cancel). Its late result is ignored.
    /// Returns the operation to cancel at the backend.
    pub fn abandon(&mut self) -> Option<LibOpId> {
        self.progress = None;
        self.active.take().map(|(id, _)| id)
    }

    /// Forget per-instance results when the player selects another instance.
    pub fn clear_instance_views(&mut self) {
        self.search = None;
        self.project = None;
        self.plan = None;
        self.worlds = None;
        self.installed = None;
        self.profile = None;
        self.integrity = None;
    }

    pub fn apply(&mut self, event: LibEvent) {
        match event {
            LibEvent::Changed(snap) => self.snap = snap,
            LibEvent::Progress { op, progress } => {
                if self.active.as_ref().is_some_and(|(id, _)| *id == op) {
                    self.progress = Some(progress);
                }
            }
            LibEvent::Finished { op, result } => {
                if self.active.as_ref().is_none_or(|(id, _)| *id != op) {
                    return; // stale: abandoned or never ours
                }
                let Some((_, command)) = self.active.take() else {
                    return;
                };
                self.progress = None;
                self.finish(command, result);
            }
        }
    }

    fn finish(&mut self, command: LibCommand, result: Result<LibPayload, LibError>) {
        let payload = match result {
            Ok(p) => p,
            Err(e) if e.kind == LibErrorKind::Cancelled => {
                self.set_notice(LibNotice::Cancelled);
                return;
            }
            Err(error) => {
                let during = kind_of(&command);
                self.set_notice(LibNotice::Error { during, error });
                return;
            }
        };
        match payload {
            LibPayload::Search(page) => {
                // Accept only the page for the query we sent.
                if matches!(&command, LibCommand::Search(q) if *q == page.query) {
                    self.search = Some(page);
                }
            }
            LibPayload::Project(details) => {
                self.plan = None;
                self.project = Some(details);
            }
            LibPayload::Worlds { instance, worlds } => self.worlds = Some((instance, worlds)),
            LibPayload::Plan(plan) => self.plan = Some(plan),
            LibPayload::Installed { kind, titles, .. } => {
                self.plan = None;
                self.set_notice(LibNotice::Installed { kind, titles });
            }
            LibPayload::InstalledList {
                instance,
                kind,
                items,
            } => self.installed = Some((instance, kind, items)),
            LibPayload::Profile {
                id,
                profile,
                revision,
            } => {
                let saved = matches!(command, LibCommand::SaveProfile { .. });
                self.profile = Some((id, profile, revision));
                if saved {
                    self.set_notice(LibNotice::ProfileSaved);
                }
            }
            LibPayload::Integrity { instance, report } => {
                let resolved = matches!(command, LibCommand::Resolve { .. });
                self.integrity = Some((instance, report));
                if resolved {
                    self.set_notice(LibNotice::IntegrityResolved);
                }
            }
            LibPayload::Created(_) => {}
            LibPayload::Done => match command {
                LibCommand::SaveTheme(_) => self.set_notice(LibNotice::ThemeSaved),
                LibCommand::SetEnabled { .. }
                | LibCommand::Update { .. }
                | LibCommand::Remove { .. } => self.set_notice(LibNotice::ContentChanged),
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(text: &str) -> SearchQuery {
        SearchQuery {
            instance: InstanceId("a".repeat(32)),
            kind: ContentKind::Mod,
            text: text.into(),
            sort: SortBy::Relevance,
            categories: Vec::new(),
            offset: 0,
        }
    }

    fn page(q: SearchQuery) -> LibPayload {
        LibPayload::Search(SearchPage {
            query: q,
            hits: Vec::new(),
            total: 0,
            categories: Vec::new(),
        })
    }

    #[test]
    fn late_results_from_abandoned_operations_are_ignored() {
        let mut m = LibraryModel::new(LibrarySnapshot::default(), None);
        m.on_submitted(LibCommand::Search(query("old")), Ok(LibOpId(1)));
        assert_eq!(m.abandon(), Some(LibOpId(1)));
        m.on_submitted(LibCommand::Search(query("new")), Ok(LibOpId(2)));
        m.apply(LibEvent::Progress {
            op: LibOpId(1),
            progress: LibProgress {
                step: LibStep::Contacting,
                done: 0,
                total: None,
                item: None,
            },
        });
        assert!(m.progress.is_none(), "progress from the old op is ignored");
        m.apply(LibEvent::Finished {
            op: LibOpId(1),
            result: Ok(page(query("old"))),
        });
        assert!(m.search.is_none() && m.busy());
        m.apply(LibEvent::Finished {
            op: LibOpId(2),
            result: Ok(page(query("new"))),
        });
        assert_eq!(
            m.search.as_ref().map(|p| p.query.text.as_str()),
            Some("new")
        );
        assert!(!m.busy());
    }

    #[test]
    fn commit_boundary_ends_cancellation_and_install_is_not_called_working() {
        let mut m = LibraryModel::new(LibrarySnapshot::default(), None);
        m.on_submitted(
            LibCommand::Apply {
                plan_id: "7".into(),
            },
            Ok(LibOpId(3)),
        );
        assert!(m.cancellable());
        m.apply(LibEvent::Progress {
            op: LibOpId(3),
            progress: LibProgress {
                step: LibStep::Committing,
                done: 1,
                total: Some(1),
                item: None,
            },
        });
        assert!(!m.cancellable());
        m.apply(LibEvent::Finished {
            op: LibOpId(3),
            result: Ok(LibPayload::Installed {
                instance: InstanceId("a".repeat(32)),
                kind: ContentKind::Shader,
                titles: vec!["Pack".into()],
            }),
        });
        assert!(matches!(m.notice, Some(LibNotice::Installed { .. })));
    }

    #[test]
    fn rejections_and_failures_become_typed_notices() {
        let mut m = LibraryModel::new(LibrarySnapshot::default(), None);
        m.on_submitted(
            LibCommand::Search(query("x")),
            Err(LibError::new(LibErrorKind::Busy)),
        );
        assert!(matches!(
            m.notice,
            Some(LibNotice::Error { during: LibOpKind::Search, ref error }) if error.kind == LibErrorKind::Busy
        ));
        m.on_submitted(
            LibCommand::Apply {
                plan_id: "1".into(),
            },
            Ok(LibOpId(9)),
        );
        m.apply(LibEvent::Finished {
            op: LibOpId(9),
            result: Err(LibError::new(LibErrorKind::Cancelled)),
        });
        assert_eq!(m.notice, Some(LibNotice::Cancelled));
        assert!(!m.busy());
    }
}
