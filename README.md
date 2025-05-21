# Natural language detectors comparison

Look inside the [`accuracy` folder](https://github.com/RoDmitry/lang_detectors_compare/tree/main/accuracy).

### Speed

| Language count | Alphabet detector | Langram max trigrams  | Langram all ngrams | Lingua high | Whatlang | Whichlang |
| --------------- | ------- | -------- | -------- | -------- | ------- | ------- |
|  16             |         |  0.479 s |  0.725 s |          |         | 0.026 s |
|  69             |         |  3.795 s |  6.919 s |          | 8.219 s |
|  74             |         |  6.239 s | 11.913 s | 58.387 s |
| 201 (unlimited) | 1.904 s | 38.64 s  | 68.956 s |

> Note: 69 are different languages from 74, so they are fast detected by `alphabet_detector`.
CPU: Intel 9700K.

### Texts source

Uses [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`.

`for file in *; do head -n 2000 "$file" > "../lang_detectors_compare/texts/${file}"; done`
