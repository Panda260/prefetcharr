use std::future::{Ready, ready};

use tracing::debug;

use crate::media_server::NowPlaying;

pub fn users(users: &[String]) -> impl FnMut(&NowPlaying) -> Ready<bool> {
    move |np: &NowPlaying| {
        let accept =
            users.is_empty() || users.contains(&np.user.id) || users.contains(&np.user.name);
        if !accept {
            debug!(
                "\n--------------------------------------------------------------------------------\n\
                 ⛔ REJECTED SESSION\n\
                 ▶ Reason:  Unwanted user\n\
                 ▶ User:    {}\n\
                 ▶ Series:  {:?}\n\
                 ▶ Config:  {:?}\n\
                 --------------------------------------------------------------------------------",
                np.user.name, np.series, users
            );
        }
        ready(accept)
    }
}

pub fn ignore_users(ignored: &[String]) -> impl FnMut(&NowPlaying) -> Ready<bool> {
    move |np: &NowPlaying| {
        let accept = !ignored.contains(&np.user.id) && !ignored.contains(&np.user.name);
        if !accept {
            debug!(
                "\n--------------------------------------------------------------------------------\n\
                 ⛔ REJECTED SESSION\n\
                 ▶ Reason:  Ignored user\n\
                 ▶ User:    {}\n\
                 ▶ Series:  {:?}\n\
                 ▶ Config:  {:?}\n\
                 --------------------------------------------------------------------------------",
                np.user.name, np.series, ignored
            );
        }
        ready(accept)
    }
}

pub fn libraries(libraries: &[String]) -> impl FnMut(&NowPlaying) -> Ready<bool> {
    move |np: &NowPlaying| {
        let library = np.library.as_ref();
        let accept = libraries.is_empty() || library.is_some_and(|l| libraries.contains(l));
        if !accept {
            debug!(
                "\n--------------------------------------------------------------------------------\n\
                 ⛔ REJECTED SESSION\n\
                 ▶ Reason:  Unwanted library\n\
                 ▶ Library: {}\n\
                 ▶ Series:  {:?}\n\
                 ▶ Config:  {:?}\n\
                 --------------------------------------------------------------------------------",
                np.library.as_deref().unwrap_or("None"),
                np.series,
                libraries
            );
        }
        ready(accept)
    }
}

#[cfg(test)]
mod test {
    use crate::media_server::{NowPlaying, User};

    fn np_default() -> NowPlaying {
        NowPlaying {
            series: crate::media_server::Series::Tvdb(0),
            episode: 0,
            season: 0,
            user: User {
                name: String::new(),
                id: String::new(),
            },
            library: None,
            session_id: None,
            item_path: None,
        }
    }

    // Empty user list accepts all sessions
    #[tokio::test]
    async fn users_unrestricted() {
        let mut filter = super::users(&[]);
        assert!(filter(&np_default()).await);
    }

    // Sessions matching a configured user name are accepted
    #[tokio::test]
    async fn users_accepted_by_name() {
        let users = vec!["Other".to_string(), "User".to_string()];
        let mut filter = super::users(users.as_slice());
        let np = NowPlaying {
            user: User {
                id: "1".to_string(),
                name: "User".to_string(),
            },
            ..np_default()
        };
        assert!(filter(&np).await);
    }

    // Sessions matching a configured user ID are accepted
    #[tokio::test]
    async fn users_accepted_by_id() {
        let users = vec!["1".to_string(), "2".to_string()];
        let mut filter = super::users(users.as_slice());
        let np = NowPlaying {
            user: User {
                id: "1".to_string(),
                name: "User".to_string(),
            },
            ..np_default()
        };
        assert!(filter(&np).await);
    }

    // Sessions not matching any configured user are rejected
    #[tokio::test]
    async fn users_rejected() {
        let users = vec!["Nope".to_string()];
        let mut filter = super::users(users.as_slice());
        let np = NowPlaying { ..np_default() };
        assert!(!filter(&np).await);
    }

    // Empty ignore list accepts all sessions
    #[tokio::test]
    async fn ignore_users_unrestricted() {
        let mut filter = super::ignore_users(&[]);
        assert!(filter(&np_default()).await);
    }

    // Sessions matching an ignored user name are rejected
    #[tokio::test]
    async fn ignore_users_rejected_by_name() {
        let ignored = vec!["Other".to_string(), "User".to_string()];
        let mut filter = super::ignore_users(ignored.as_slice());
        let np = NowPlaying {
            user: User {
                id: "1".to_string(),
                name: "User".to_string(),
            },
            ..np_default()
        };
        assert!(!filter(&np).await);
    }

    // Sessions matching an ignored user ID are rejected
    #[tokio::test]
    async fn ignore_users_rejected_by_id() {
        let ignored = vec!["1".to_string(), "2".to_string()];
        let mut filter = super::ignore_users(ignored.as_slice());
        let np = NowPlaying {
            user: User {
                id: "1".to_string(),
                name: "User".to_string(),
            },
            ..np_default()
        };
        assert!(!filter(&np).await);
    }

    // Sessions not matching any ignored user are accepted
    #[tokio::test]
    async fn ignore_users_accepted() {
        let ignored = vec!["Nope".to_string()];
        let mut filter = super::ignore_users(ignored.as_slice());
        let np = NowPlaying {
            user: User {
                id: "1".to_string(),
                name: "User".to_string(),
            },
            ..np_default()
        };
        assert!(filter(&np).await);
    }

    // Empty library list accepts all sessions
    #[tokio::test]
    async fn libraries_unrestricted() {
        let mut filter = super::libraries(&[]);
        assert!(filter(&np_default()).await);
    }

    // Sessions from a matching library are accepted
    #[tokio::test]
    async fn libraries_accepted() {
        let libraries = vec!["Movies".to_string(), "TV".to_string()];
        let mut filter = super::libraries(libraries.as_slice());
        let np = NowPlaying {
            library: Some("TV".to_string()),
            ..np_default()
        };
        assert!(filter(&np).await);
    }

    // Sessions with no library set are rejected when a library filter is active
    #[tokio::test]
    async fn libraries_unknown_rejected() {
        let libraries = vec!["Nope".to_string()];
        let mut filter = super::libraries(libraries.as_slice());
        let np = NowPlaying {
            library: None,
            ..np_default()
        };
        assert!(!filter(&np).await);
    }

    // Sessions from a non-matching library are rejected
    #[tokio::test]
    async fn libraries_rejected() {
        let libraries = vec!["TV".to_string()];
        let mut filter = super::libraries(libraries.as_slice());
        let np = NowPlaying {
            library: Some("Movies".to_string()),
            ..np_default()
        };
        assert!(!filter(&np).await);
    }
}
