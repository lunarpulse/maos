URL: https://platform.kimi.ai/docs/pricing/chat.md
Content-Type: text/markdown
Method: text

---

> ## Documentation Index
> Fetch the complete documentation index at: https://platform.kimi.ai/docs/llms.txt
> Use this file to discover all available pages before exploring further.

# Model Inference Pricing

> Understand token billing, input and output charges, cache discounts, and pricing links for Kimi model inference.

## Model Pricing

### K3 Series Models

All K3 series models support a 1,048,576-token context window.

<table>
  <thead>
    <tr />
  </thead>

  <tbody />
</table>

Note: The Kimi API **automatically enables** context caching billing for repeated request prefixes, with a default TTL (time-to-live) of 5m. For TTL settings and cost optimization tips, see [Best Practices for Context Caching](https://platform.kimi.ai/docs/guide/context-caching.md).

**[Batch API Inference](https://platform.kimi.ai/docs/pricing/batch.md)**: `kimi-k3.1-pro` and `kimi-k3` support batch inference for large-scale tasks that do not require real-time responses, billed at 60% of the standard price.

### K2 Series Models

All K2 series models support a 262,144-token context window.

> [!NOTE]
>
> Dear Kimi Users, starting October 31, 2026, K2 series models will no longer be available to newly registered users; existing users are not affected. We plan to retire all K2 series models by the end of December 2026, please [prepare your migration](https://platform.kimi.ai/docs/guide/kimi-k3-1-pro-quickstart.md) in advance.

<table>
  <thead>
    <tr />
  </thead>

  <tbody />
</table>

1 MTok = 1,000,000 tokens. All prices above are per MTok consumed.

**Explanation: Prices exclude applicable taxes. Specific tax obligations are subject to local tax regulations and will be calculated at checkout based on your jurisdiction.**

## Concepts

> [!NOTE]
>
> Model inference on the Kimi API platform is billed on a pay-as-you-go basis, with no subscription plan. It differs from products such as the Kimi membership and Kimi Code. [See the product comparison](https://platform.kimi.ai/docs/guide/product-plans.md).

### Billing Unit

Token: the basic unit of text processed by the model. A long word like "antidisestablishmentarianism" may be split into several tokens, while a common short word like "word" usually takes just one. For typical English text, 1 token is roughly equivalent to 3-4 characters. Use the [Token Calculation API](https://platform.kimi.ai/docs/api/estimate.md) to get the exact token count of each call.

### Billing Logic

Model inference APIs bill input and output tokens by usage. For K3 series models, cache writes are billed separately by TTL (5min / 1h); cached input is billed at the cache-hit price only, with no additional cache write charge. If you upload a document, extract its content, and pass the extracted content as model input, that content is also billed by usage. File-related APIs (file content extraction and file storage) are **temporarily free**: uploading and extracting a document alone incurs no file API charges. For tips on improving cache hit rates and choosing a TTL tier, see [Best Practices for Context Caching](https://platform.kimi.ai/docs/guide/context-caching.md).

## Model Notes

See each model's guide for capabilities and recommended use cases:

* [Kimi K3.1 Pro](https://platform.kimi.ai/docs/guide/kimi-k3-1-pro-quickstart.md)
* [Kimi K3](https://platform.kimi.ai/docs/guide/kimi-k3-quickstart.md)
* [Kimi K2.7 Code](https://platform.kimi.ai/docs/guide/kimi-k2-7-code-quickstart.md)
* [Kimi K2.6](https://platform.kimi.ai/docs/guide/kimi-k2-6-quickstart.md)