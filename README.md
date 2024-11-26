# Ical Property

This crate is an addition to the [ical](https://docs.rs/ical/latest/ical/)
crate.
It takes an `ical::IcalEvent` and
derives an `ical_property::Event`.
The `ical::IcalEvent` has properties, but they
are more or less a `Vec<(&str, &str)>`,
which makes it tedious to obtain information from it.

The the heart of this crate, is the `ical_property::Event`.
It should contain all fields, an entry in a typical
ical calender has, like `uid`, `summary`,
`attendees`, information about recurrence, etc.
The `Event` struct implements `TryFrom`
for `ical::IcalEvent`.

