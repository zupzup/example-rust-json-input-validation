# example-rust-json-input-validation

An example for doing basic JSON input validation with Rust, using warp.

Run with `cargo run` and then:


For basic, and improved JSON parse handling:

```bash
curl -X POST http://localhost:8080/create-basic -H "Content-Type: application/json" -d '{ "email": 1, "address": { "street": "warpstreet", "street_no": 1 }, "pets": [{ "name": "nacho" }] }'

curl -X POST http://localhost:8080/create-path -H "Content-Type: application/json" -d '{ "email": 1, "address": { "street": "warpstreet", "street_no": 1 }, "pets": [{ "name": "nacho" }] }'
```

And for validating the outcoming struct:

```bash
curl -X POST http://localhost:8080/create-validator -H "Content-Type: application/json" -d '{ "email": "chip@example.com", "address": { "street": "warpstreet", "street_no": 1 }, "pets": [{ "name": "" }] }'
```
