// From the smallest to the largest: the largest change since the last release decides the next version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    Internal,
    Fix,
    Feature,
    Breaking,
}

impl Change {
    pub fn of_commit(message: &str) -> Option<Self> {
        let subject = message.lines().next().unwrap_or_default();
        let (type_and_scope, _) = subject.split_once(':').unwrap_or_default();
        if type_and_scope.ends_with('!') || message.lines().any(|line| line.starts_with("BREAKING CHANGE:")) {
            return Some(Self::Breaking);
        }
        let commit_type = type_and_scope.split('(').next().unwrap_or_default();
        match commit_type {
            "docs" => None,
            "feat" => Some(Self::Feature),
            "fix" | "perf" => Some(Self::Fix),
            _ => Some(Self::Internal),
        }
    }

    pub fn is_noticed_by_users(self) -> bool {
        self != Self::Internal
    }
}
