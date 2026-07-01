---
type: BigQuery Table
title: GA4 Events
description: Synthetic ecommerce events table for interoperability smoke testing.
resource: https://bigquery.example/projects/demo/datasets/analytics/tables/events
tags:
  - analytics
  - bigquery
  - ecommerce
timestamp: "2026-06-19T12:00:00Z"
partitioned: true
producer:
  sample: synthetic
  system: bigquery
row_count: 123456
---
| column | type | notes |
| --- | --- | --- |
| event_date | DATE | Partition date |
| event_name | STRING | Event identifier |
| user_pseudo_id | STRING | Pseudonymous user key |
| event_params | RECORD | Repeated event parameters |

Joins:

- `user_pseudo_id` to [Users](users.md)
- `event_params.key` to [Parameter Dictionary](../reference/event-params.md)
