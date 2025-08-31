//! ドメインモデル（イベントソーシング/CQRS）
//!
//! アーキテクチャ文書の「付録A」に対応する最小実装です。
//! Robert C. Martin の SOLID 原則のうち、単一責任（SRP）を意識し、
//! 本モジュールはドメイン状態遷移（イベント適用）のみを担当します。

/// Todo に関するイベント種別
/// （付録Aの Event に相当）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// 作成（タイトルを設定し、completed=false で開始）
    Created { title: String },
    /// タイトル変更
    TitleChanged { title: String },
    /// 完了
    Completed,
    /// 再開（未完了へ戻す）
    Reopened,
    /// 削除（トゥームストーン）
    Deleted,
}

/// Todo に対するコマンド種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Create { title: String },
    ChangeTitle { title: String },
    Complete,
    Reopen,
    Delete,
}

/// ドメインエラー（不変条件違反など）
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("already created")] 
    AlreadyCreated,
    #[error("aggregate is not created yet")] 
    NotCreated,
    #[error("aggregate is deleted (tombstoned)")] 
    Deleted,
}

/// Todo 集約の状態
/// （付録Aの Todo に相当）
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Todo {
    /// タイトル
    pub title: String,
    /// 完了フラグ
    pub completed: bool,
    /// 削除フラグ（tombstone）
    pub deleted: bool,
    /// 集約のバージョン（適用済みイベント数）
    pub version: u64,
}

impl Todo {
    /// コマンドから発生するイベントを決定します。
    /// 不変条件に反する場合は `DomainError` を返します。
    pub fn decide(&self, cmd: Command) -> Result<Vec<Event>, DomainError> {
        use Command::*;
        if self.deleted {
            return Err(DomainError::Deleted);
        }

        match cmd {
            Create { title } => {
                if self.version > 0 {
                    return Err(DomainError::AlreadyCreated);
                }
                Ok(vec![Event::Created { title }])
            }
            ChangeTitle { title } => {
                if self.version == 0 {
                    return Err(DomainError::NotCreated);
                }
                Ok(vec![Event::TitleChanged { title }])
            }
            Complete => {
                if self.version == 0 {
                    return Err(DomainError::NotCreated);
                }
                Ok(vec![Event::Completed])
            }
            Reopen => {
                if self.version == 0 {
                    return Err(DomainError::NotCreated);
                }
                Ok(vec![Event::Reopened])
            }
            Delete => {
                if self.version == 0 {
                    return Err(DomainError::NotCreated);
                }
                Ok(vec![Event::Deleted])
            }
        }
    }

    /// イベントを 1 件適用して状態を遷移させます。
    /// 常に `version` を +1 します（冪等性は呼び出し側で制御）。
    pub fn apply(&mut self, ev: &Event) {
        match ev {
            Event::Created { title } => {
                self.title = title.clone();
                self.completed = false;
                self.deleted = false;
            }
            Event::TitleChanged { title } => {
                self.title = title.clone();
            }
            Event::Completed => {
                self.completed = true;
            }
            Event::Reopened => {
                self.completed = false;
            }
            Event::Deleted => {
                // 物理削除は行わず、tombstone として扱う
                self.deleted = true;
            }
        }
        self.version = self.version.saturating_add(1);
    }

    /// 一連のイベントから状態を再構築します（Event Sourcing の基本操作）。
    pub fn from_events<I>(events: I) -> Self
    where
        I: IntoIterator<Item = Event>,
    {
        let mut s = Self::default();
        for ev in events {
            s.apply(&ev);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_created_sets_title_and_clears_flags() {
        let mut todo = Todo::default();
        todo.apply(&Event::Created { title: "Buy milk".into() });
        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.completed);
        assert!(!todo.deleted);
        assert_eq!(todo.version, 1);
    }

    #[test]
    fn complete_and_reopen_flip_completed_flag() {
        let mut todo = Todo::default();
        todo.apply(&Event::Created { title: "Task".into() });
        todo.apply(&Event::Completed);
        assert!(todo.completed);
        todo.apply(&Event::Reopened);
        assert!(!todo.completed);
        assert_eq!(todo.version, 3);
    }

    #[test]
    fn deleted_sets_tombstone_flag() {
        let mut todo = Todo::default();
        todo.apply(&Event::Created { title: "Task".into() });
        todo.apply(&Event::Deleted);
        assert!(todo.deleted);
        assert_eq!(todo.version, 2);
    }

    // プロパティベーステスト: 適用イベント数 = version の性質
    // 注: ビジネス制約（例: Deleted 後に変更不可）は別レイヤで検証する想定
    mod prop {
        use super::*;
        use proptest::prelude::*;

        // 任意のイベントを生成するストラテジ
        fn any_event() -> impl Strategy<Value = Event> {
            let title = ".{0,64}"; // タイトルは 0..64 文字
            prop_oneof![
                // Created
                proptest::string::string_regex(title).unwrap().prop_map(|s| Event::Created { title: s }),
                // TitleChanged
                proptest::string::string_regex(title).unwrap().prop_map(|s| Event::TitleChanged { title: s }),
                // Completed/Reopened/Deleted
                Just(Event::Completed),
                Just(Event::Reopened),
                Just(Event::Deleted),
            ]
        }

        proptest! {
            #[test]
            fn version_always_equals_number_of_applied_events(events in proptest::collection::vec(any_event(), 0..50)) {
                let t = Todo::from_events(events.clone());
                prop_assert_eq!(t.version as usize, events.len());
            }
        }
    }

    #[test]
    fn decide_create_then_apply_yields_created() {
        let todo = Todo::default();
        let evs = todo.decide(Command::Create { title: "X".into() }).unwrap();
        assert_eq!(evs, vec![Event::Created { title: "X".into() }]);

        let mut t = Todo::default();
        for ev in &evs { t.apply(ev); }
        assert_eq!(t.title, "X");
        assert_eq!(t.version, 1);
    }

    #[test]
    fn decide_disallows_create_twice() {
        let mut t = Todo::default();
        // 1回作成
        let evs = t.decide(Command::Create { title: "A".into() }).unwrap();
        for ev in &evs { t.apply(ev); }
        // 2回目はエラー
        let err = t.decide(Command::Create { title: "B".into() }).unwrap_err();
        assert_eq!(err, DomainError::AlreadyCreated);
    }

    #[test]
    fn decide_requires_created_before_updates() {
        let t = Todo::default();
        assert_eq!(t.decide(Command::ChangeTitle { title: "A".into() }).unwrap_err(), DomainError::NotCreated);
        assert_eq!(t.decide(Command::Complete).unwrap_err(), DomainError::NotCreated);
        assert_eq!(t.decide(Command::Reopen).unwrap_err(), DomainError::NotCreated);
        assert_eq!(t.decide(Command::Delete).unwrap_err(), DomainError::NotCreated);
    }

    #[test]
    fn decide_after_deleted_is_error() {
        // 作成→削除
        let t = Todo::from_events([
            Event::Created { title: "X".into() },
            Event::Deleted,
        ]);
        assert_eq!(t.version, 2);
        // 以後の操作は全てエラー
        assert_eq!(t.decide(Command::ChangeTitle { title: "Y".into() }).unwrap_err(), DomainError::Deleted);
        assert_eq!(t.decide(Command::Complete).unwrap_err(), DomainError::Deleted);
        assert_eq!(t.decide(Command::Reopen).unwrap_err(), DomainError::Deleted);
        assert_eq!(t.decide(Command::Delete).unwrap_err(), DomainError::Deleted);
    }
}
