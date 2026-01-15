# Natural language detectors comparison

Look inside the [`accuracy` folder](https://github.com/RoDmitry/lang_detectors_compare/tree/main/accuracy).

### Detection Speed (Time)

| Languages count | Alphabet detector | Langram max trigrams  | Langram all ngrams | Lingua high | Whatlang | Whichlang |
| --------------- | ------- | -------- | -------- | -------- | ------- | ------- |
|  16             |         |  0.681 s |  1.006 s |          |         | 0.034 s |
|  70             |         |  3.385 s |  5.000 s |          | 5.244 s |
|  74             |         |  4.369 s |  6.504 s | 54.513 s |
| 201 (unlimited) | 2.081 s | 14.225 s | 20.202 s |

> Note: 70 are different languages from 74, so they are fast detected by `alphabet_detector`.
CPU: Intel 9700K.

### Texts source

Uses [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`, `san` to `cls`.
Removed `taqTfng`.

`for file in *; do head -n 2000 "$file" > "../lang_detectors_compare/texts/${file}"; done`
