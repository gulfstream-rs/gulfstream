# Monetization, ads, and administration

This page describes how Gulfstream is prepared for paying creators per
confirmed view, and the recommended design for the remaining pieces (ad
delivery, ledger, admin panel). Only the measurement foundation ships today.

## Shipped foundation: qualified views

A **qualified view** is a playback session whose *server-measured* watch time
reaches `analytics.qualified_view_seconds` (default 30 s), or
`qualified_view_minimum_fraction` (default 50 %) of shorter videos. It is
counted once per viewer, media, and UTC day, so reloading a page does not
multiply earnings. Watch time can only grow as fast as real time and playback
progress, request and session rates are limited per client, and each session
records its country and surface (`watch` or `embed`).

Qualified views are exposed per day, per media, and per country by the
analytics API (`qualified_views` fields, `/analytics/countries`) and are the
unit a payout ledger should consume.

## Inserting ads

Do not burn ads into the video files: that requires re-encoding, cannot be
targeted or rotated, and cannot be counted reliably. Use one of these instead:

1. **Client-side ads (recommended first step).** The player requests a
   [VAST 4 / VMAP](https://iabtechlab.com/standards/vast/) document from an ad
   server and plays pre-roll, mid-roll (at cue points), and post-roll creatives
   in an overlay `<video>` before resuming content. Use the Google IMA HTML5
   SDK (works with Google Ad Manager and any VAST server) or an open-source
   VAST client. Ad impressions and quartiles are reported by the SDK to the ad
   server; the player should also report `ad_impression` events to Gulfstream
   so the ledger can count ads per qualified view. Ad blockers can block this.
2. **Server-side ad insertion (SSAI).** Ads are transcoded to the same ladder
   and stitched into the HLS playlist with `#EXT-X-DISCONTINUITY` (or
   interstitials via `EXT-X-DATERANGE`) per session. Resistant to ad blockers
   and smoother, but requires per-session manifests, ad transcoding, and
   server-side beaconing. Consider it once client-side ads work.

Mid-roll positions can be chosen per media (manual cue points) or
automatically (for example every 8 minutes, only for videos longer than 10
minutes) and stored with the media record.

## Earnings model

1. A daily job aggregates qualified views and *verified ad impressions* per
   media and country.
2. Earnings = Σ impressions × CPM ÷ 1000 × creator revenue share, with CPM
   rates per country tier (country data is already collected).
3. Results are written to an append-only, double-entry **ledger**
   (`ledger_entries`: account, amount in minor units, currency, source period,
   idempotency key). The balance is the sum of entries — never a mutable
   column. Late fraud adjustments are separate negative entries.
4. Earnings stay *pending* for a holdback period (for example 30 days) to allow
   invalid-traffic review, then become *available*; payouts debit the ledger.

## Is an admin panel required?

Yes, once money is involved. It should be a separate, role-protected area (not
the creator dashboard) with:

- **Users:** list, search, suspend, storage quotas, roles, KYC/payout status.
- **Content:** all media across accounts, takedown/moderation, ad eligibility.
- **Statistics:** platform-wide views, qualified views, countries, bandwidth,
  top creators and media, invalid-traffic signals (views per client, embed
  domains, abnormal completion patterns).
- **Ads:** campaigns or ad-server tags, VAST/VMAP endpoints, frequency caps,
  ad breaks policy, country CPM tables, creator revenue share.
- **Finance:** ledger browser, holdbacks, adjustments, payout batches, exports.
- **Audit log:** every administrative action, with actor and reason.

Prerequisites in the core: an account `role` (`creator`/`admin`), admin-only
API routes, and an audit table. Keep admin actions on API-key or session
authentication with CSRF protection, as the rest of the API does.
