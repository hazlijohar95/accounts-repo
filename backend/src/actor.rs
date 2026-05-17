use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticatedActor {
    pub auth_user_id: String,
    pub display_name: String,
    pub email: String,
}
