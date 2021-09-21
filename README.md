# ready-image

Kubernetes controller to keep the specified container images ready on every node.
An updated version of [`warm-image`] using [`kube`].

```yaml
kind: ReadyImage
apiVersion: qualified.io/v1alpha1
metadata:
  name: example
  # namespace: readyimage-system
spec:
  image: "alpine:3.14"
  # imagePullSecrets:
  #   - name: secret
  # nodeSelector:
  #   readyImage: 'true'
```

Creating the above resource creates a `DaemonSet` with the specified image that does nothing.

## Usage

TBD

## TODO?

Instead of creating a new `DaemonSet` when `image` is updated, patch the existing `DaemonSet`?

<details>
<summary>request body of <code>kubectl set image</code></summary>

`Content-Type: application/strategic-merge-patch+json`
```json
{
  "spec": {
    "template": {
      "spec": {
        "$setElementOrder/containers": [
          { "name": "container-name" }
        ],
        "containers": [
          { "name": "container-name", "image": "new-image" }
        ]
      }
    }
  }
}
```

</details>

---

Instead of using `initContainers` to copy `sleeper` into a memfs volume, create `ConfigMap` with `binaryData` and mount that as an executable? [`hang`] is about 14KB.

<details>
<summary>YAML</summary>

```yaml
kind: ConfigMap
apiVersion: v1
metadata:
  name: hang
  namespace: default
binaryData:
  hang: '...'
```
```yaml
# PodSpec
containers:
- name: image
  image: readyimage.spec.image
  command: ['/hang']
  volumeMounts:
  - name: hang
    mountPath: /hang
    subPath: hang
volumes:
- name: hang
  configMap:
    name: hang
    defaultMode: 0755
```

</details>

[`warm-image`]: https://github.com/mattmoor/warm-image
[`kube`]: https://github.com/kube-rs/kube-rs
[`hang`]: https://github.com/nathan-osman/hang
