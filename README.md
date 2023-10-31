# CosmWasm MultiTest

## Overview

**CosmWasm MultiTest** is a suite of test helpers for multi-contract interactions.

## Warning

**CosmWasm MultiTest** is currently in **alpha** stage, designed generally for internal use only.

**`Use at your own risk`**

Internally, **CosmWasm MultiTest** is used for testing cw-plus contracts.
We have no API stability yet. We are working on refactoring it,
and will expose a more refined version for use in other contracts.

**CosmWasm MultiTest** can be used to run unit tests with contracts calling contracts,
and calling in and out of bank. **CosmWasm MultiTest** works with contracts and bank currently.
We are working on making it more extensible for more handlers,
including custom messages/queries, as well as IBC.
