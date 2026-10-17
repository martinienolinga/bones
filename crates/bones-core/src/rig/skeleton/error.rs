//! Erreurs de structure du squelette (messages affichés en anglais).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkeletonError {
    /// L'os `bone` référence un parent qui n'apparaît pas avant lui.
    InvalidParent { bone: usize },
    /// Le nouveau parent est l'os lui-même ou l'un de ses descendants.
    Cycle,
    /// Identifiant d'os inexistant.
    UnknownBone,
}

impl std::fmt::Display for SkeletonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkeletonError::InvalidParent { bone } => write!(f, "bone {bone} has an invalid parent"),
            SkeletonError::Cycle => {
                write!(f, "a bone cannot be parented to itself or to one of its children")
            }
            SkeletonError::UnknownBone => write!(f, "unknown bone"),
        }
    }
}

impl std::error::Error for SkeletonError {}
