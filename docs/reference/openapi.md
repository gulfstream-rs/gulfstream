# OpenAPI

Gulfstream ships an OpenAPI 3.1 template at the repository root. At runtime the server resolves its configured base URL, route prefixes, registration header, CSRF header, documentation URL, repository URL, and package version before serving the document.

Default location:

```text
http://localhost:8080/openapi.yaml
```

Download it:

```bash
curl -o gulfstream-openapi.yaml http://localhost:8080/openapi.yaml
```

Use this runtime document when generating clients because route prefixes are configurable. The source template is also available in the [repository](https://github.com/gulfstream-rs/gulfstream/blob/main/openapi.yaml).
