# Natural language detectors comparison

Look inside the [`accuracy` folder](https://github.com/RoDmitry/lang_detectors_compare/tree/main/accuracy).

### Speed

| Language count | Alphabet detector | Langram max trigrams  | Langram all ngrams | Lingua high | Whatlang | Whichlang |
| --------------- | ------- | -------- | -------- | -------- | ------- | ------- |
|  16             |         |  0.516 s |  0.695 s |          |         | 0.026 s |
|  69             |         |  3.646 s |  5.535 s |          | 8.219 s |
|  74             |         |  5.794 s |  9.272 s | 58.387 s |
| 201 (unlimited) | 1.901 s | 32.272 s | 48.143 s |

> Note: 69 are different languages from 74, so they are fast detected by `alphabet_detector`.
CPU: Intel 9700K.

### Texts source

Uses [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`.

`for file in *; do head -n 2000 "$file" > "../lang_detectors_compare/texts/${file}"; done`
