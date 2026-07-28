# Mocking guide

When you mock a payment call, the example looks like this:

```bash
curl -X POST https://api.example.com/charge -d "token=$STRIPE_KEY"
```

That is illustrative only; nothing here is executed by the skill.
