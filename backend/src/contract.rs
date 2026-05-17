use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiContract {
    pub version: u32,
    pub routes: Vec<ApiRoute>,
    pub enums: Vec<ApiEnum>,
    pub dto_interfaces: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiRoute {
    pub method: &'static str,
    pub path: &'static str,
    pub request: Option<&'static str>,
    pub response: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApiEnum {
    pub name: &'static str,
    pub values: Vec<&'static str>,
}

pub fn api_contract() -> ApiContract {
    ApiContract {
        version: 1,
        routes: vec![
            ApiRoute {
                method: "GET",
                path: "/api/meta/contract",
                request: None,
                response: "ApiContract",
            },
            ApiRoute {
                method: "GET",
                path: "/api/repos",
                request: None,
                response: "LegalEntityRepo[]",
            },
            ApiRoute {
                method: "POST",
                path: "/api/imports/year-end-review-pack",
                request: Some("ImportWorkspacePayload"),
                response: "RepoWorkspace",
            },
            ApiRoute {
                method: "GET",
                path: "/api/repos/{repo_id}",
                request: None,
                response: "RepoWorkspace",
            },
            ApiRoute {
                method: "GET",
                path: "/api/repos/{repo_id}/audit",
                request: None,
                response: "AuditEvent[]",
            },
            ApiRoute {
                method: "POST",
                path: "/api/repos/{repo_id}/branches/{branch_id}/correction-commits",
                request: Some("CorrectionCommitPayload"),
                response: "Commit",
            },
            ApiRoute {
                method: "GET",
                path: "/api/review-packs/{review_pack_id}",
                request: None,
                response: "ReviewPack",
            },
            ApiRoute {
                method: "POST",
                path: "/api/review-packs/{review_pack_id}/reviewer-approval",
                request: Some("ApprovalPayload"),
                response: "Approval",
            },
            ApiRoute {
                method: "POST",
                path: "/api/review-packs/{review_pack_id}/client-signoff",
                request: Some("ApprovalPayload"),
                response: "Approval",
            },
            ApiRoute {
                method: "POST",
                path: "/api/review-packs/{review_pack_id}/queries",
                request: Some("ReviewQueryPayload"),
                response: "ReviewQuery",
            },
            ApiRoute {
                method: "POST",
                path: "/api/review-packs/{review_pack_id}/queries/{query_id}/resolve",
                request: Some("ResolveReviewQueryPayload"),
                response: "ReviewQuery",
            },
            ApiRoute {
                method: "POST",
                path: "/api/review-packs/{review_pack_id}/signed-export",
                request: None,
                response: "SignedPackExport",
            },
        ],
        enums: vec![
            ApiEnum {
                name: "ReviewStatus",
                values: vec!["in_review", "reviewer_approved", "signed"],
            },
            ApiEnum {
                name: "BranchStatus",
                values: vec!["working", "in_review", "frozen"],
            },
            ApiEnum {
                name: "RepoRole",
                values: vec!["owner", "preparer", "reviewer", "client_signer", "observer"],
            },
        ],
        dto_interfaces: vec![
            "LegalEntityRepo",
            "RepoWorkspace",
            "Commit",
            "ReviewPack",
            "Approval",
            "ReviewQuery",
            "AuditEvent",
            "SignedPackExport",
            "ImportWorkspacePayload",
            "CorrectionCommitPayload",
            "ReviewQueryPayload",
            "ResolveReviewQueryPayload",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::api_contract;

    #[test]
    fn frontend_types_cover_backend_contract_to_prevent_drift() {
        let frontend_types = include_str!("../../frontend/src/types.ts");
        let contract = api_contract();

        for interface in contract.dto_interfaces {
            assert!(
                frontend_types.contains(&format!("interface {interface}")),
                "frontend types are missing interface {interface}",
            );
        }

        for api_enum in contract.enums {
            assert!(
                frontend_types.contains(&format!("type {}", api_enum.name)),
                "frontend types are missing enum {}",
                api_enum.name,
            );
            for value in api_enum.values {
                assert!(
                    frontend_types.contains(&format!("\"{value}\"")),
                    "frontend enum {} is missing value {value}",
                    api_enum.name,
                );
            }
        }
    }
}
