// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
import { AsyncJob, AsyncEffect, __purrdf_test_open_exchange } from './fixture';

const effect: AsyncEffect = __purrdf_test_open_exchange(9_007_199_254_740_993n);
const id: bigint | undefined = effect.exchangeId;
if (id !== undefined) {
  AsyncJob.exchangeIsOpen(id);
  AsyncJob.deliverExchangeBindings(id, new Uint8Array());
  AsyncJob.deliverExchangeFailure(id, 'transport', 'down');
  AsyncJob.faultExchange(id, 'handler fault');
}
// @ts-expect-error exchange identities cannot travel through number.
AsyncJob.exchangeIsOpen(1);
// @ts-expect-error settlement requires the exact bigint identity.
AsyncJob.deliverExchangeBindings(1, new Uint8Array());
// @ts-expect-error failure cannot settle a numeric identity.
AsyncJob.deliverExchangeFailure(1, 'transport', 'down');
// @ts-expect-error a fault cannot settle a numeric identity.
AsyncJob.faultExchange(1, 'fault');
