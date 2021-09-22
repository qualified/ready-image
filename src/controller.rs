use std::collections::BTreeMap;

use futures::StreamExt;
use k8s_openapi::{
    api::{
        apps::v1::{DaemonSet, DaemonSetSpec},
        core::v1::{
            Container, EmptyDirVolumeSource, PodSpec, PodTemplateSpec, ResourceRequirements,
            Volume, VolumeMount,
        },
    },
    apimachinery::pkg::{
        api::resource::Quantity,
        apis::meta::v1::{LabelSelector, OwnerReference},
    },
};
use kube::{
    api::{DeleteParams, ListParams, ObjectMeta, PostParams, PropagationPolicy, ResourceExt},
    Api, Client, Resource,
};
use kube_runtime::controller::{Context, Controller, ReconcilerAction};
use snafu::{ResultExt, Snafu};

use super::ReadyImage;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to list DaemonSet: {}", source))]
    List { source: kube::Error },
    #[snafu(display("failed to create DaemonSet: {}", source))]
    Create { source: kube::Error },
    #[snafu(display("failed to delete DaemonSets: {}", source))]
    Delete { source: kube::Error },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Data to store in context
struct ContextData {
    client: Client,
    sleeper_image: String,
}

pub async fn run(client: Client, sleeper_image: String) {
    let context = Context::new(ContextData {
        client: client.clone(),
        sleeper_image,
    });

    let lp = ListParams::default();
    Controller::<ReadyImage>::new(Api::default_namespaced(client.clone()), lp.clone())
        .owns::<DaemonSet>(Api::default_namespaced(client.clone()), lp.clone())
        .shutdown_on_signal()
        .run(reconciler, error_policy, context)
        .filter_map(|x| async move { x.ok() })
        .for_each(|(_, action)| async move {
            tracing::trace!("Reconciled: requeue after {:?}", action.requeue_after);
        })
        .await;
}

#[tracing::instrument(skip(rimg, ctx), level = "trace")]
async fn reconciler(rimg: ReadyImage, ctx: Context<ContextData>) -> Result<ReconcilerAction> {
    // TODO Conditions
    let client = ctx.get_ref().client.clone();
    let daemons: Api<DaemonSet> = Api::default_namespaced(client);
    let uid = rimg.uid().expect("uid set");
    let version = rimg.resource_version().expect("resource version set");
    let list = daemons
        .list(&ListParams {
            label_selector: Some(format!(
                "{}={},{}={}",
                LABEL_CONTROLLER, &uid, LABEL_VERSION, &version
            )),
            ..ListParams::default()
        })
        .await
        .context(List)?;
    match list.items.len() {
        0 => {
            // create
            let ds = build_daemon_set(&rimg, &ctx.get_ref().sleeper_image);
            match daemons.create(&PostParams::default(), &ds).await {
                Ok(_) => {
                    tracing::trace!("DaemonSet created");
                }
                Err(kube::Error::Api(kube::error::ErrorResponse { code: 409, .. })) => {
                    tracing::trace!("DaemonSet already existed");
                }
                Err(source) => return Err(Error::Create { source }),
            }
        }

        1 => {
            // just clean up any old ones.
            tracing::trace!("up to date");
        }

        _ => {
            // something caused multiple `DaemonSet` to exist, remove except one.
            tracing::warn!(
                "multiple DaemonSet found for {}={},{}={}",
                LABEL_CONTROLLER,
                &uid,
                LABEL_VERSION,
                &version
            );
            let mut dss = list.items.iter();
            // Keep the first one
            dss.next();
            let delete_param = DeleteParams {
                propagation_policy: Some(PropagationPolicy::Foreground),
                // Delete immediately
                grace_period_seconds: Some(0),
                ..DeleteParams::default()
            };
            for ds in dss {
                let name = ds.name();
                match daemons.delete(&name, &delete_param).await {
                    Ok(_) => {
                        tracing::trace!("DaemonSet {} deleted", &name);
                    }
                    Err(kube::Error::Api(kube::error::ErrorResponse { code: 404, .. })) => {
                        tracing::trace!("DaemonSet {} was already deleted", &name);
                    }
                    Err(source) => {
                        tracing::trace!("failed to delete {}", &name);
                        return Err(Error::Delete { source });
                    }
                }
            }
        }
    }

    // Clean up any old DaemonSets
    match daemons
        .delete_collection(
            &DeleteParams {
                propagation_policy: Some(PropagationPolicy::Foreground),
                // Delete immediately
                grace_period_seconds: Some(0),
                ..DeleteParams::default()
            },
            &ListParams {
                label_selector: Some(format!(
                    "{}={},{}!={}",
                    LABEL_CONTROLLER, &uid, LABEL_VERSION, &version
                )),
                ..ListParams::default()
            },
        )
        .await
        .context(Delete)?
    {
        either::Left(list) => {
            if !list.items.is_empty() {
                tracing::trace!(
                    "deleting old DaemonSets: {:?}",
                    list.iter().map(ResourceExt::name).collect::<Vec<_>>()
                );
            }
        }
        either::Right(status) => {
            tracing::trace!("deleting old DaemonSets: {:?}", status);
        }
    }

    Ok(ReconcilerAction {
        requeue_after: None,
    })
}

#[allow(clippy::needless_pass_by_value)]
/// An error handler called when the reconciler fails.
fn error_policy(error: &Error, _ctx: Context<ContextData>) -> ReconcilerAction {
    tracing::warn!("reconciler failed: {}", error);
    ReconcilerAction {
        requeue_after: None,
    }
}

fn to_owner_reference(rimg: &ReadyImage) -> OwnerReference {
    OwnerReference {
        api_version: ReadyImage::api_version(&()).into_owned(),
        kind: ReadyImage::kind(&()).into_owned(),
        name: rimg.name(),
        uid: rimg.uid().expect(".metadata.uid"),
        controller: Some(true),
        block_owner_deletion: Some(true),
    }
}

fn build_daemon_set(rimg: &ReadyImage, sleeper_image: &str) -> DaemonSet {
    let labels = make_labels(rimg);
    let volume = Volume {
        name: "sleeper".into(),
        // memfs
        empty_dir: Some(EmptyDirVolumeSource {
            medium: Some("Memory".into()),
            size_limit: Some(Quantity("5Mi".into())),
        }),
        ..Volume::default()
    };
    DaemonSet {
        metadata: ObjectMeta {
            // Generate a unique name with prefix
            name: None,
            generate_name: Some(format!("{}-", rimg.name())),
            labels: Some(labels.clone()),
            owner_references: Some(vec![to_owner_reference(rimg)]),
            ..ObjectMeta::default()
        },
        spec: Some(DaemonSetSpec {
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels.clone()),
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..ObjectMeta::default()
                }),
                spec: Some(PodSpec {
                    init_containers: Some(vec![Container {
                        name: "copy-sleeper".into(),
                        image: Some(sleeper_image.into()),
                        command: Some(
                            vec!["/sleeper", "/drop/sleeper"]
                                .into_iter()
                                .map(str::to_owned)
                                .collect::<Vec<_>>(),
                        ),
                        volume_mounts: Some(vec![VolumeMount {
                            name: volume.name.clone(),
                            mount_path: "/drop/".into(),
                            ..VolumeMount::default()
                        }]),
                        resources: Some(ResourceRequirements {
                            limits: Some(
                                vec![
                                    ("cpu".to_owned(), Quantity("1m".into())),
                                    ("memory".to_owned(), Quantity("20M".into())),
                                ]
                                .into_iter()
                                .collect::<BTreeMap<_, _>>(),
                            ),
                            ..ResourceRequirements::default()
                        }),
                        ..Container::default()
                    }]),
                    containers: vec![Container {
                        name: "image".into(),
                        image: Some(rimg.spec.image.clone()),
                        image_pull_policy: Some("Always".into()),
                        command: Some(vec!["/drop/sleeper".into()]),
                        volume_mounts: Some(vec![VolumeMount {
                            name: volume.name.clone(),
                            mount_path: "/drop/".into(),
                            ..VolumeMount::default()
                        }]),
                        resources: Some(ResourceRequirements {
                            limits: Some(
                                vec![
                                    ("cpu".to_owned(), Quantity("1m".into())),
                                    ("memory".to_owned(), Quantity("20M".into())),
                                ]
                                .into_iter()
                                .collect::<BTreeMap<_, _>>(),
                            ),
                            ..ResourceRequirements::default()
                        }),
                        ..Container::default()
                    }],
                    image_pull_secrets: rimg.spec.image_pull_secrets.clone(),
                    volumes: Some(vec![volume]),
                    node_selector: rimg.spec.node_selector.clone(),
                    ..PodSpec::default()
                }),
            },
            ..DaemonSetSpec::default()
        }),
        ..DaemonSet::default()
    }
}

const LABEL_CONTROLLER: &str = "readyimage/controller";
const LABEL_VERSION: &str = "readyimage/version";

fn make_labels(rimg: &ReadyImage) -> BTreeMap<String, String> {
    vec![
        (LABEL_CONTROLLER, rimg.uid().expect(".metadata.uid")),
        (
            LABEL_VERSION,
            rimg.resource_version().expect("resource_ver"),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}
