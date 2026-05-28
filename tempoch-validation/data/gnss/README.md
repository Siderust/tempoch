# GNSS ICD reference points

This directory contains check points derived from the published ICDs:

- `gps_epoch.csv` — GPS week 0, second 0 = 1980-01-06 00:00:00 UTC.
  Source: IS-GPS-200, Rev. R, §3.3.4 (System Time).
- `bdt_epoch.csv` — BDT week 0, second 0 = 2006-01-01 00:00:00 UTC.
  Source: BeiDou ICD-OS Version 2.1, §5.2.
- `galileo_epoch.csv` — GST week 0, second 0 = 1999-08-22 00:00:00 UTC
  (i.e. midnight between Saturday and Sunday). Source: OS-SIS-ICD Issue 2.1,
  §5.1.2.
- `gps_week_rollover.csv` — week 1024 rollover (1999-08-22 → 1999-08-22 UTC) and
  week 2048 rollover (2019-04-07 → 2019-04-07 UTC). Source: IS-GPS-200.

CSV schema (all GNSS epoch files):

```
label,scale,utc_iso,tai_minus_utc_s,nominal_tai_minus_scale_s
```

- `label` — human-readable description of the check point.
- `scale` — `GPST`, `GST`, `BDT`, or `QZSST`.
- `utc_iso` — civil UTC label (ISO 8601 `YYYY-MM-DDTHH:MM:SSZ`).
- `tai_minus_utc_s` — leap seconds in effect at this UTC instant (per the
  IERS UTC-TAI history).
- `nominal_tai_minus_scale_s` — nominal fixed offset (19 s for GPST/GST/QZSST,
  33 s for BDT).

All values are exact integers per the ICDs at the respective epochs.
