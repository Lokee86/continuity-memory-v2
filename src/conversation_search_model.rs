use crate::Fragment;

#[derive(Clone, Debug, PartialEq)]
pub struct ConversationSearchHit {
    pub fragment: Fragment,
    pub text: String,
    pub score: f64,
}
