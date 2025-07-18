# Pipeline

The `Pipeline` represents a data processing job (a simpler version of this is called [MonoVertex](./monovertex.md)). The
most important concept in Numaflow, it defines:

1. A list of [vertices](vertex.md), which define the data processing tasks;
1. A list of `edges`, which are used to describe the relationship between the vertices. Note an edge may go from a vertex
   to multiple vertices, and an edge may also go from multiple vertices to a vertex. This many-to-one relationship is
   possible via [Join and Cycles](../user-guide/reference/join-vertex.md)

The `Pipeline` is abstracted as a [Kubernetes Custom Resource](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/). A `Pipeline` spec looks like below.

```yaml
apiVersion: numaflow.numaproj.io/v1alpha1
kind: Pipeline
metadata:
  name: simple-pipeline
spec:
  vertices:
    - name: in
      source:
        generator:
          rpu: 5
          duration: 1s
    - name: cat
      udf:
        container:
          image: quay.io/numaio/numaflow-go/map-cat:stable # A UDF which simply cats the message
          imagePullPolicy: Always
    - name: out
      sink:
        log: {}
  edges:
    - from: in
      to: cat
    - from: cat
      to: out
```

To query `Pipeline` objects with `kubectl`:

```sh
kubectl get pipeline # or "pl" as a short name
```
