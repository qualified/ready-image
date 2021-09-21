use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    namespaced,
    group = "qualified.io",
    version = "v1alpha1",
    kind = "ReadyImage",
    plural = "readyimages",
    shortname = "rimg",
    shortname = "rimgs"
)]
#[serde(rename_all = "camelCase")]
pub struct ReadyImageSpec {
    /// The container image to keep ready on nodes.
    pub image: String,

    /// Optional list of references to secrets to use for pulling the container image.
    /// Secrets must be in the same namespace as the controller because Pods are created there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_pull_secrets: Option<Vec<k8s_openapi::api::core::v1::LocalObjectReference>>,

    /// Optional node selector to select the nodes to keep the image ready. Defaults to all nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<std::collections::BTreeMap<String, String>>,
}
