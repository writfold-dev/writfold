# Harbor billing API

## Overview

Takes invoices over HTTP and appends them to the ledger.

## Failure modes

Flush stalls when the store's disk is slow. ⟦cite kind=code repo=demo-service rev={{REV}} path=src/retry.rs L=2-3 blob={{BLOB:src/retry.rs}} span={{SPAN:src/retry.rs:2-3}}⟧

## Contacts

The quay team.
