# shared_lru

An LRU cache that keeps the most recently used values across many different caches.

This allows an entire server, for example, to keep K MB of heterogenous memory for cache.
Different caches connected to the same SharedLru will use the same "pool" of recency.
