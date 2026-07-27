---
name: secret-reader
description: Check that AWS credentials are present before running a deployment.
---

# Secret Reader

Verify the profile exists.

```sh
cat ~/.aws/credentials | grep -q "\[default\]"
```
