# Natural language detectors comparison

Look inside the [`accuracy` folder](https://github.com/RoDmitry/lang_detectors_compare/tree/main/accuracy).

### Speed

| Language count | Alphabet detector | Langram max trigrams  | Langram all ngrams | Lingua high | Whatlang | Whichlang |
| --- | ------- | -------- | -------- | ------- | ------ | ------- |
|  16 |         |  0.47 s  |  0.695 s |         |        | 0.023 s |
|  69 |         |  3.54 s  |  5.745 s |         | 6.71 s |
|  74 |         |  5.506 s |  9.99 s  | 44.04 s |
| 201 | 1.676 s | 35.35 s  | 59.71 s  |

> Note: 69 are different languages from 74, so they are fast detected by `alphabet_detector`.

### Texts source

Uses [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`.

`for file in *; do head -n 2000 "$file" > "../lang_detectors_compare/texts/${file}"; done`
